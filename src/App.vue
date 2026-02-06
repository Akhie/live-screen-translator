<script setup>
import { ref, computed, onMounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const enabled = ref(false);
const selecting = ref(false);

const start = ref({ x: 0, y: 0 });
const current = ref({ x: 0, y: 0 });

const preview = ref(null);
const webviewOffset = ref({ x: 0, y: 0 });

const capturedRect = ref(null);
const overlayText = ref(null);

const lines = ref([]);

onMounted(() => {
  const rect = document.body.getBoundingClientRect();
  webviewOffset.value = {
    x: rect.left,
    y: rect.top
  };
});

/* Start / Stop */

async function startSelection() {
  enabled.value = true;
  selecting.value = false;
  await appWindow.setIgnoreCursorEvents(false);
}

async function stopSelection() {
  enabled.value = false;
  selecting.value = false;
  await appWindow.setIgnoreCursorEvents(true);
}

/* Mouse events (screen coords ONLY) */

function onMouseDown(e) {
  if (!enabled.value) return;
  selecting.value = true;
  start.value = { x: e.screenX, y: e.screenY };
  current.value = { x: e.screenX, y: e.screenY };
}

function onMouseMove(e) {
  if (!selecting.value) return;
  current.value = { x: e.screenX, y: e.screenY };
}

async function onMouseUp() {
  if (!selecting.value) return;
  selecting.value = false;

  await nextTick();
  await showPreView();
  extractLines();
}

async function showPreView() {
  // 🔥 DPI FIX (this is the key)
  const dpr = window.devicePixelRatio || 1;

  const logicalX = Math.min(start.value.x, current.value.x);
  const logicalY = Math.min(start.value.y, current.value.y);
  const logicalW = Math.abs(start.value.x - current.value.x);
  const logicalH = Math.abs(start.value.y - current.value.y);

  // Convert logical → physical pixels for Rust
  const x = Math.round(logicalX * dpr);
  const y = Math.round(logicalY * dpr);
  const width = Math.round(logicalW * dpr);
  const height = Math.round(logicalH * dpr);

  if (width < 5 || height < 5) return;

  try {
    const bytes = await invoke("capture_area", { x, y, width, height });

    const base64 = `data:image/png;base64,${btoa(
      new Uint8Array(bytes).reduce((a, b) => a + String.fromCharCode(b), "")
    )}`;

    preview.value = {
      src: base64,
      x: logicalX,
      y: logicalY,
      w: logicalW,
      h: logicalH
    };
    // send this image to backend and get the translation
    capturedRect.value = { x, y, width, height };
    //overlayText.value = "Hello from backend";
  } catch (e) {
    console.error("Capture failed:", e);
  }
}

/* ---------------- logic ---------------- */
function extractLines() {
  const height = Math.abs(start.value.y - current.value.y);

  const lineHeight = 20;
  const count = Math.max(1, Math.floor(height / lineHeight));

  const top = Math.min(start.value.y, current.value.y);
  const left = Math.min(start.value.x, current.value.x);

  const newLines = Array.from({ length: count }).map((_, i) => ({
    text: `Translated text here ${i + 1}`,
    x: left + 6,
    y: top + i * lineHeight - 25,
  }));

  lines.value = [...lines.value, ...newLines];
}

const textStyle = computed(() => {
  if (!capturedRect.value) return {};

  const rect = document.body.getBoundingClientRect();

  return {
    left:
      capturedRect.value.x -
      window.screenX -
      rect.left +
      "px",

    top:
      capturedRect.value.y -
      window.screenY -
      rect.top +
      "px",

    width: capturedRect.value.width + "px",
    height: capturedRect.value.height + "px"
  };
});

function refreshOverlay() {
  enabled.value = false;
  selecting.value = false;

  start.value = { x: 0, y: 0 };
  current.value = { x: 0, y: 0 };

  capturedRect.value = null;
  overlayText.value = "";
  lines.value = [];

  console.log("Overlay refreshed");
}


/* PERFECT box positioning */

const selectionStyle = computed(() => {
  const left =
    Math.min(start.value.x, current.value.x) -
    (window.screenX + webviewOffset.value.x);

  const top =
    Math.min(start.value.y, current.value.y) -
    (window.screenY + webviewOffset.value.y);

  const width = Math.abs(start.value.x - current.value.x);
  const height = Math.abs(start.value.y - current.value.y);

  return {
    left: left + "px",
    top: top + "px",
    width: width + "px",
    height: height + "px"
  };
});


</script>

<template>
  <div class="controls">
    <button @click="startSelection">Start</button>
    <button @click="stopSelection">Stop</button>
    <button @click="refreshOverlay">Refresh</button>
  </div>

  <div class="overlay" :class="{ active: enabled }" @mousedown="onMouseDown" @mousemove="onMouseMove"
    @mouseup="onMouseUp">
    <div v-if="selecting" class="selection" :style="selectionStyle" />
  </div>

  <!-- rendered text -->
  <div v-for="(line, i) in lines" :key="i" class="overlay-text" :style="{
    left: line.x + 'px',
    top: line.y + 'px'
  }">
    {{ line.text }}
  </div>

  <!-- Render captured image EXACTLY on top -->
  <!-- <img
    v-if="preview"
    :src="preview.src"
    class="preview"
    :style="{
      left: preview.x + 'px',
      top: preview.y + 'px',
      width: preview.w + 'px',
      height: preview.h + 'px'
    }"
  /> -->
  <div v-if="preview" class="preview">
    <h3>Captured Preview:</h3>
    <img :src="preview.src" />
  </div>
  <div v-if="overlayText" class="text-overlay" :style="{
    left: preview.x + 'px',
    top: preview.y + 'px',
    width: preview.w + 'px',
    height: preview.h + 'px'
  }">
    {{ overlayText }}
  </div>
</template>

<style>
.controls {
  position: fixed;
  top: 10px;
  left: 10px;
  z-index: 10001;
}

.overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 10000;
}

.overlay.active {
  pointer-events: auto;
  cursor: crosshair;
}

.selection {
  position: absolute;
  border: 2px dashed #00aaff;
  background: rgba(0, 170, 255, 0.25);
}

.overlay-text {
  position: absolute;
  font-size: 14px;
  line-height: 20px;
  color: red;
  white-space: nowrap;
  pointer-events: none;
}

.preview {
  position: fixed;
  bottom: 10px;
  left: 10px;
  z-index: 10002;
  background: white;
  padding: 10px;
  border: 1px solid #ccc;
}

.preview img {
  max-width: 300px;
  max-height: 200px;
}

.text-overlay {
  position: fixed;
  background: transparent;
  color: #fff;
  font-size: 18px;
  padding: 8px;
  box-sizing: border-box;
  z-index: 10002;
  pointer-events: none;
  white-space: pre-wrap;
}
</style>