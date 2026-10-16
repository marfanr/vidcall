/// <reference types="vite/client" />

declare module "@braid/vue-formulate" {
  import { DefineComponent } from "vue";

  export const FormulateForm: DefineComponent<{
    modelValue?: Record<string, unknown>;
    class?: string;
    "onSubmit"?: (values: Record<string, unknown>) => void;
  }>;

  export const FormulateInput: DefineComponent<{
    type?: string;
    name?: string;
    autocomplete?: string;
    placeholder?: string;
    validation?: string;
    "validation-messages"?: Record<string, string>;
    "input-class"?: string[];
    modelValue?: string;
  }>;
}
