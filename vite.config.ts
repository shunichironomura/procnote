import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
  lint: {},
  plugins: [sveltekit()],
  server: {
    port: 5180,
    strictPort: true,
  },
});
