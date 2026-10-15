<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const videoRef = ref<HTMLVideoElement | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)

const hueCenter = ref(225)
const hueRange = ref(30)
const hueShift = ref(0)
const saturation = ref(1.4)
const brightness = ref(1.0)
const strength = ref(1.0)

const status = ref('Initializing camera...')
const gpuAvailable = ref(false)

let stream: MediaStream | null = null
let device: GPUDevice | null = null
let context: GPUCanvasContext | null = null
let uniformBuffer: GPUBuffer | null = null
let videoTexture: GPUTexture | null = null
let sampler: GPUSampler | null = null
let pipeline: GPURenderPipeline | null = null
let bindGroup: GPUBindGroup | null = null
let animationId = 0
let running = false
let lastTextureWidth = 0
let lastTextureHeight = 0

const shaderCode = /* wgsl */ `
struct Params {
  // x = hue center, y = hue range, z = hue shift, w = effect strength
  hue: vec4f,
  // x = saturation multiplier, y = brightness multiplier
  adjustments: vec4f,
}

@group(0) @binding(0) var videoTexture: texture_2d<f32>;
@group(0) @binding(1) var videoSampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
}

@vertex
fn vertexMain(@builtin(vertex_index) index: u32) -> VertexOutput {
  // Full-screen triangle.
  var positions = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f( 3.0, -1.0),
    vec2f(-1.0,  3.0)
  );

  var out: VertexOutput;
  let p = positions[index];
  out.position = vec4f(p, 0.0, 1.0);
  out.uv = vec2f(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  return out;
}

fn rgbToHsv(c: vec3f) -> vec3f {
  let maxC = max(c.r, max(c.g, c.b));
  let minC = min(c.r, min(c.g, c.b));
  let delta = maxC - minC;

  var h = 0.0;
  if (delta > 0.00001) {
    if (maxC == c.r) {
      h = (c.g - c.b) / delta;
      if (c.g < c.b) {
        h += 6.0;
      }
    } else if (maxC == c.g) {
      h = (c.b - c.r) / delta + 2.0;
    } else {
      h = (c.r - c.g) / delta + 4.0;
    }
    h /= 6.0;
  }

  var s = 0.0;
  if (maxC > 0.00001) {
    s = delta / maxC;
  }
  return vec3f(h, s, maxC);
}

fn hsvToRgb(c: vec3f) -> vec3f {
  let k = vec3f(1.0, 2.0 / 3.0, 1.0 / 3.0);
  let p = abs(fract(c.xxx + k) * 6.0 - 3.0);
  return c.z * mix(
    vec3f(1.0),
    clamp(p - vec3f(1.0), vec3f(0.0), vec3f(1.0)),
    c.y
  );
}

@fragment
fn fragmentMain(in: VertexOutput) -> @location(0) vec4f {
  let color = textureSample(videoTexture, videoSampler, in.uv);
  var hsv = rgbToHsv(color.rgb);

  let center = params.hue.x;
  let range = params.hue.y;
  let shift = params.hue.z;
  let amount = clamp(params.hue.w, 0.0, 1.0);

  // Hue values are normalized to [0, 1]. Use circular distance at the wrap boundary.
  let rawDistance = abs(hsv.x - center);
  let distance = min(rawDistance, 1.0 - rawDistance);
  let feather = max(2.0 / 360.0, range * 0.15);
  let lower = max(0.0, range - feather);
  let mask = 1.0 - smoothstep(lower, max(lower + 0.0001, range), distance);
  let influence = mask * amount;

  hsv.x = fract(hsv.x + shift * influence);
  hsv.y = clamp(
    hsv.y * mix(1.0, params.adjustments.x, influence),
    0.0,
    1.0
  );
  hsv.z = clamp(
    hsv.z * mix(1.0, params.adjustments.y, influence),
    0.0,
    1.0
  );

  return vec4f(hsvToRgb(hsv), color.a);
}
`

function updateUniforms() {
  if (!device || !uniformBuffer) return

  device.queue.writeBuffer(
    uniformBuffer,
    0,
    new Float32Array([
      hueCenter.value / 360,
      hueRange.value / 360,
      hueShift.value / 360,
      strength.value,
      saturation.value,
      brightness.value,
      0,
      0,
    ]),
  )
}

function createVideoTexture(width: number, height: number) {
  if (!device || !pipeline || !uniformBuffer || !sampler) return

  videoTexture?.destroy()
  videoTexture = device.createTexture({
    label: 'camera-frame-texture',
    size: { width, height, depthOrArrayLayers: 1 },
    format: 'rgba8unorm',
    // copyExternalImageToTexture requires RENDER_ATTACHMENT on the destination.
    usage:
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.RENDER_ATTACHMENT,
  })
  lastTextureWidth = width
  lastTextureHeight = height

  // The bind group only depends on the texture, so rebuild it here instead of every frame.
  bindGroup = device.createBindGroup({
    label: 'color-grading-bind-group',
    layout: pipeline.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: videoTexture.createView() },
      { binding: 1, resource: sampler },
      { binding: 2, resource: { buffer: uniformBuffer } },
    ],
  })
}

function stopRendering() {
  running = false
  cancelAnimationFrame(animationId)
}

async function initWebGPU() {
  if (!navigator.gpu) {
    status.value = 'WebGPU is not supported in this browser/context'
    return
  }

  const canvas = canvasRef.value
  const video = videoRef.value
  if (!canvas || !video) {
    status.value = 'Video or canvas element is unavailable'
    return
  }

  status.value = 'Requesting GPU adapter...'
  const adapter = await navigator.gpu.requestAdapter()
  if (!adapter) {
    status.value = 'No GPU adapter available'
    return
  }

  device = await adapter.requestDevice()
  device.addEventListener('uncapturederror', (event) => {
    console.error('Uncaptured WebGPU error:', event.error)
    // Stop the loop so a persistent error doesn't flood the console every frame.
    stopRendering()
    status.value = `GPU error: ${event.error.message}`
  })

  device.lost.then((info) => {
    if (running) {
      stopRendering()
      status.value = `GPU device lost: ${info.message || info.reason}`
    }
  })

  context = canvas.getContext('webgpu')
  if (!context) {
    status.value = 'Could not create WebGPU canvas context'
    return
  }

  const format = navigator.gpu.getPreferredCanvasFormat()
  context.configure({
    device,
    format,
    alphaMode: 'opaque',
  })

  status.value = 'Compiling color-grading shader...'
  const shader = device.createShaderModule({
    label: 'color-grading-shader',
    code: shaderCode,
  })

  const compilation = await shader.getCompilationInfo()
  const shaderErrors = compilation.messages.filter((message) => message.type === 'error')
  for (const message of compilation.messages) {
    const log = message.type === 'error' ? console.error : console.warn
    log(`[WGSL ${message.type}] line ${message.lineNum}: ${message.message}`)
  }
  if (shaderErrors.length > 0) {
    status.value = `WGSL compilation failed (${shaderErrors.length} error(s)); check console`
    return
  }

  pipeline = await device.createRenderPipelineAsync({
    label: 'color-grading-pipeline',
    layout: 'auto',
    vertex: {
      module: shader,
      entryPoint: 'vertexMain',
    },
    fragment: {
      module: shader,
      entryPoint: 'fragmentMain',
      targets: [{ format }],
    },
    primitive: {
      topology: 'triangle-list',
    },
  })

  uniformBuffer = device.createBuffer({
    label: 'color-grading-uniforms',
    size: 32,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  })

  sampler = device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
    addressModeU: 'clamp-to-edge',
    addressModeV: 'clamp-to-edge',
  })

  gpuAvailable.value = true
  status.value = 'WebGPU ready; waiting for camera frame'
  running = true
  render()
}

function render() {
  if (!running || !device || !context || !pipeline || !uniformBuffer || !sampler) return
  animationId = requestAnimationFrame(render)

  const video = videoRef.value
  const canvas = canvasRef.value
  if (!video || !canvas || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) return

  const width = video.videoWidth
  const height = video.videoHeight
  if (width < 1 || height < 1) return

  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width
    canvas.height = height
  }

  try {
    if (!videoTexture || !bindGroup || width !== lastTextureWidth || height !== lastTextureHeight) {
      createVideoTexture(width, height)
    }
    if (!videoTexture || !bindGroup) return

    // Copy the current camera frame to a regular 2D GPU texture.
    // This avoids browser-specific texture_external binding issues.
    device.queue.copyExternalImageToTexture(
      { source: video },
      { texture: videoTexture },
      { width, height, depthOrArrayLayers: 1 },
    )

    updateUniforms()

    const encoder = device.createCommandEncoder()
    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    })

    pass.setPipeline(pipeline)
    pass.setBindGroup(0, bindGroup)
    pass.draw(3)
    pass.end()
    device.queue.submit([encoder.finish()])

    if (status.value !== 'WebGPU active · color grading running') {
      status.value = 'WebGPU active · color grading running'
    }
  } catch (error) {
    console.error('GPU render error:', error)
    stopRendering()
    status.value = `Render error: ${error instanceof Error ? error.message : String(error)}`
  }
}

function resetGrading() {
  hueCenter.value = 225
  hueRange.value = 30
  hueShift.value = 0
  saturation.value = 1.4
  brightness.value = 1.0
  strength.value = 1.0
}

function setPreset(center: number, range: number) {
  hueCenter.value = center
  hueRange.value = range
}

onMounted(async () => {
  try {
    if (!navigator.mediaDevices?.getUserMedia) {
      status.value = 'Camera API unavailable; use HTTPS or localhost'
      return
    }

    status.value = 'Requesting camera permission...'
    stream = await navigator.mediaDevices.getUserMedia({
      video: {
        facingMode: 'user',
        width: { ideal: 1280 },
        height: { ideal: 720 },
      },
      audio: false,
    })

    const video = videoRef.value
    if (!video) {
      status.value = 'Video element is unavailable'
      return
    }

    video.srcObject = stream
    await new Promise<void>((resolve) => {
      if (video.readyState >= HTMLMediaElement.HAVE_METADATA) {
        resolve()
      } else {
        video.addEventListener('loadedmetadata', () => resolve(), { once: true })
      }
    })
    await video.play()
    await initWebGPU()
  } catch (error) {
    console.error('Initialization failed:', error)
    status.value = `Initialization failed: ${error instanceof Error ? error.message : String(error)}`
  }
})

onUnmounted(() => {
  stopRendering()

  stream?.getTracks().forEach((track) => track.stop())
  bindGroup = null
  videoTexture?.destroy()
  uniformBuffer?.destroy()
  device?.destroy()
})
</script>

<template>
  <main class="min-h-screen bg-slate-950 p-6 text-white md:p-8">
    <header class="mb-6">
      <h1 class="text-2xl font-semibold">WebGPU Color Grading</h1>
      <p class="mt-2 text-sm text-slate-400">
        Selective HSV color correction · WGSL shader
      </p>
      <p class="mt-3 flex items-center gap-2 text-sm" aria-live="polite">
        <span class="inline-block h-2 w-2 rounded-full" :class="gpuAvailable ? 'bg-emerald-400' : 'bg-amber-400'" />
        {{ status }}
      </p>
    </header>

    <section class="grid grid-cols-1 gap-5 md:grid-cols-2">
      <div>
        <h2 class="mb-2 text-sm text-slate-400">Original camera</h2>
        <video ref="videoRef" autoplay playsinline muted
          class="aspect-video w-full rounded-xl bg-black object-contain" />
      </div>

      <div>
        <h2 class="mb-2 text-sm text-emerald-400">GPU graded output</h2>
        <canvas ref="canvasRef" class="aspect-video w-full rounded-xl bg-black object-contain" />
      </div>
    </section>

    <section class="mt-8 max-w-3xl rounded-xl border border-slate-800 p-5">
      <div class="mb-6 flex items-center justify-between gap-4">
        <div>
          <h2 class="font-medium">Color selection</h2>
          <p class="mt-1 text-xs text-slate-400">HSV hue in degrees (0–360°)</p>
        </div>
        <button class="rounded-lg bg-slate-800 px-3 py-2 text-sm hover:bg-slate-700" @click="resetGrading">
          Reset
        </button>
      </div>

      <div class="space-y-6">
        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="hue-center">Hue Center</label>
            <span>{{ hueCenter }}°</span>
          </div>
          <input id="hue-center" v-model.number="hueCenter" type="range" min="0" max="360"
            class="w-full accent-blue-400">
        </div>

        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="hue-range">Hue Range (± degrees)</label>
            <span>±{{ hueRange }}°</span>
          </div>
          <input id="hue-range" v-model.number="hueRange" type="range" min="5" max="180" class="w-full accent-blue-400">
        </div>

        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="hue-shift">Hue Shift</label>
            <span>{{ hueShift }}°</span>
          </div>
          <input id="hue-shift" v-model.number="hueShift" type="range" min="-180" max="180"
            class="w-full accent-purple-400">
        </div>

        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="saturation">Saturation multiplier</label>
            <span>{{ saturation.toFixed(2) }}×</span>
          </div>
          <input id="saturation" v-model.number="saturation" type="range" min="0" max="2" step="0.05"
            class="w-full accent-emerald-400">
        </div>

        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="brightness">Brightness multiplier</label>
            <span>{{ brightness.toFixed(2) }}×</span>
          </div>
          <input id="brightness" v-model.number="brightness" type="range" min="0.2" max="2" step="0.05"
            class="w-full accent-amber-400">
        </div>

        <div>
          <div class="mb-2 flex justify-between text-sm">
            <label for="strength">Effect strength</label>
            <span>{{ Math.round(strength * 100) }}%</span>
          </div>
          <input id="strength" v-model.number="strength" type="range" min="0" max="1" step="0.05"
            class="w-full accent-pink-400">
        </div>
      </div>

      <div class="mt-6 flex flex-wrap gap-2">
        <button class="rounded-lg border border-blue-800 bg-blue-950 px-3 py-2 text-sm"
          @click="setPreset(225, 30)">Blue</button>
        <button class="rounded-lg border border-cyan-800 bg-cyan-950 px-3 py-2 text-sm"
          @click="setPreset(180, 20)">Cyan</button>
        <button class="rounded-lg border border-green-800 bg-green-950 px-3 py-2 text-sm"
          @click="setPreset(120, 30)">Green</button>
        <button class="rounded-lg border border-red-800 bg-red-950 px-3 py-2 text-sm"
          @click="setPreset(0, 20)">Red</button>
      </div>
    </section>
  </main>
</template>