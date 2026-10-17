<script setup lang="ts">
import { WebcamIcon } from "lucide-vue-next";
import { ref } from "vue";
import { api } from "@/lib/axios";
import { useForm, useField } from "vee-validate";
import { toTypedSchema } from "@vee-validate/zod";
import * as z from "zod";
import { useAuthStore } from "@/stores/auth";

const showPassword = ref(false);
const loading = ref(false);

const { handleSubmit, setFieldError } = useForm({
  validationSchema: toTypedSchema(
    z.object({
      email: z.string().min(1, "Email is required").email("Please enter a valid email address"),
      password: z.string().min(1, "Password is required")
    })
  )
});

const { value: email, errorMessage: emailError } = useField<string>("email");
const { value: password, errorMessage: passwordError } = useField<string>("password");

const onSubmit = handleSubmit(async (values) => {
  if (loading.value) return;
  loading.value = true;
  try {
    const res = await api.post(
      "/auth/signin",
      {
        "email": values.email,
        "password": values.password
      },
    )

    const auth = useAuthStore();
    auth.setAccessToken(res.data?.token)
  }
  catch (err) {
    const axiosErr = err as { response?: { data?: { error?: string }; status?: number } };
    const message = axiosErr.response?.data?.error || "Something went wrong";
    if (message === "invalid credentials") {
      setFieldError("email", message);
      setFieldError("password", message);
    } else if (axiosErr.response?.status === 401) {
      setFieldError("email", message);
      setFieldError("password", message);
    } else {
      setFieldError("email", message);
    }
  }
  finally {
    loading.value = false;
  }
});

</script>

<template>
  <main class="min-h-screen w-full bg-[#e9edf5] p-4 sm:p-8 text-slate-900">
    <div
      class="mx-auto grid min-h-[calc(100vh-2rem)] sm:min-h-[calc(100vh-4rem)] max-w-6xl grid-cols-1 overflow-hidden bg-white lg:grid-cols-2">
      <!-- Left panel -->
      <section class="relative m-3 hidden flex-col justify-between overflow-hidden bg-[#1d4cff] p-8 text-white lg:flex">
        <div class="flex items-center gap-2 text-xl font-semibold">
          <WebcamIcon />
          Vidcall.
        </div>

        <!-- decorative -->
        <div class="pointer-events-none absolute right-6 top-6 grid grid-cols-3 gap-1.5 opacity-40">
          <span v-for="i in 6" :key="i" class="h-4 w-0.5 bg-white/70"></span>
        </div>
        <div class="pointer-events-none absolute bottom-6 left-6 grid grid-cols-4 gap-2 opacity-30">
          <span v-for="i in 6" :key="i" class="size-1 rounded-full bg-white"></span>
        </div>



        <div class="text-center">
          <h2 class="text-3xl font-semibold tracking-tight">Speedy, Easy and Fast</h2>
          <p class="mx-auto mt-3 max-w-sm text-xs leading-relaxed text-white/70">
            Overpay helps you set saving goals, earn cash back offers, and get paychecks up to two days early.
          </p>
          <div class="mt-5 flex justify-center gap-1.5">
            <span class="size-1 rounded-full bg-white/40"></span>
            <span class="h-1 w-5 rounded-full bg-white/80"></span>
            <span class="size-1 rounded-full bg-white/40"></span>
          </div>
        </div>
      </section>

      <!-- Right panel -->
      <section class="flex flex-col justify-between p-6 sm:p-10">
        <div></div>

        <div class="mx-auto w-full max-w-sm">
          <div class="text-center">
            <h1 class="text-2xl font-semibold tracking-tight">Login to your account</h1>
            <p class="mt-2 text-sm text-slate-400">Enter your email below to login to your account</p>
          </div>

          <div class="my-6 flex items-center gap-3 text-xs text-slate-400">
            <!-- <span class="h-px flex-1 bg-slate-200"></span>
            Or with email
            <span class="h-px flex-1 bg-slate-200"></span> -->
          </div>

          <form class="flex flex-col gap-3" @submit="onSubmit">
            <div>
              <input v-model="email" type="email" autocomplete="email" placeholder="Email"
                class="h-11 w-full rounded-lg border border-slate-200 bg-white px-4 text-sm placeholder:text-slate-400 transition focus:border-[#1d4cff] focus:outline-none focus:ring-2 focus:ring-[#1d4cff]/20" />
              <p v-if="emailError" class="mt-1 text-xs text-red-500">{{ emailError }}</p>
            </div>

            <div class="relative">
              <input v-model="password" :type="showPassword ? 'text' : 'password'" autocomplete="current-password"
                placeholder="Password"
                class="h-11 w-full rounded-lg border border-slate-200 bg-white pl-4 pr-11 text-sm placeholder:text-slate-400 transition focus:border-[#1d4cff] focus:outline-none focus:ring-2 focus:ring-[#1d4cff]/20" />

              <button type="button"
                class="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-slate-400 hover:text-slate-700"
                :aria-label="showPassword ? 'Hide password' : 'Show password'" @click="showPassword = !showPassword">
                <svg v-if="!showPassword" class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                  stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M2 12s3.500-7 10-7 10 7 10 7-3.500 7-10 7S2 12 2 12z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
                <svg v-else class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                  stroke-linecap="round" stroke-linejoin="round">
                  <path
                    d="M17.940 17.940A10.940 10.940 0 0 1 12 19c-6.500 0-10-7-10-7a18.500 18.500 0 0 1 5.060-5.940M9.900 4.240A9.120 9.120 0 0 1 12 5c6.500 0 10 7 10 7a18.500 18.500 0 0 1-2.160 3.190M1 1l22 22" />
                </svg>
              </button>
              <p v-if="passwordError" class="absolute -bottom-5 left-0 text-xs text-red-500">{{ passwordError }}</p>
            </div>

            <div class="flex justify-end">
              <a href="#" class="text-xs text-slate-500 hover:text-slate-900">Forgot password?</a>
            </div>

            <button type="submit" :disabled="loading"
              class="mt-2 h-11 w-full rounded-lg bg-[#1d4cff] text-sm font-semibold text-white transition hover:bg-[#1640d9] active:scale-[0.99] disabled:opacity-60">
              {{ loading ? "Signing in..." : "Login" }}
            </button>
          </form>

          <p class="mt-6 text-center text-sm text-slate-600">
            Don't have an account?
            <a href="#" class="font-semibold text-slate-900 hover:underline">Sign Up</a>
          </p>
        </div>

        <div class="mt-10 flex justify-between text-xs text-slate-400">
          <a href="#" class="hover:text-slate-700">Privacy Policy</a>
          <span>Copyright 2026</span>
        </div>
      </section>
    </div>
  </main>
</template>
