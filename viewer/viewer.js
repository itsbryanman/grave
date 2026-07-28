const DAY_SECONDS = 86_400;
const MAX_PREVIEW_EDGE = 1_600;

const state = {
  bytes: null,
  wasm: null,
  headerInfo: null,
  latestRender: null,
  todayDay: Math.floor(Date.now() / 1000 / DAY_SECONDS),
};

const elements = {
  dropzone: document.querySelector("#dropzone"),
  input: document.querySelector("#grave-input"),
  status: document.querySelector("#status"),
  recordTitle: document.querySelector("#record-title"),
  facts: document.querySelector("#facts"),
  slider: document.querySelector("#day-slider"),
  scrubberLabel: document.querySelector("#scrubber-label"),
  buriedLabel: document.querySelector("#buried-label"),
  currentLabel: document.querySelector("#current-label"),
  terminalLabel: document.querySelector("#terminal-label"),
  futureRegion: document.querySelector("#future-region"),
  todayTick: document.querySelector("#today-tick"),
  image: document.querySelector("#image-preview"),
  text: document.querySelector("#text-preview"),
  headstone: document.querySelector("#headstone-preview"),
  emptyState: document.querySelector("#empty-state"),
};

boot().catch((error) => {
  setStatus(error.message, true);
});

async function boot() {
  wireInteractions();
  await loadWasm();
}

function wireInteractions() {
  elements.input.addEventListener("change", async (event) => {
    const [file] = event.target.files ?? [];
    if (file) {
      await loadGrave(file);
    }
  });

  elements.dropzone.addEventListener("dragover", (event) => {
    event.preventDefault();
    elements.dropzone.classList.add("is-dragging");
  });

  elements.dropzone.addEventListener("dragleave", () => {
    elements.dropzone.classList.remove("is-dragging");
  });

  elements.dropzone.addEventListener("drop", async (event) => {
    event.preventDefault();
    elements.dropzone.classList.remove("is-dragging");
    const [file] = event.dataTransfer?.files ?? [];
    if (file) {
      elements.input.files = event.dataTransfer.files;
      await loadGrave(file);
    }
  });

  elements.slider.addEventListener("input", async () => {
    if (!state.bytes || !state.wasm) {
      return;
    }
    await renderSelectedDay(Number(elements.slider.value));
  });
}

async function loadWasm() {
  try {
    const pkg = await import("./pkg/grave_wasm.js");
    await pkg.default();
    state.wasm = pkg;
    setStatus("The chapel tools are ready. Drop a grave to begin.");
  } catch (error) {
    setStatus(
      "The wasm bundle is not present in this checkout. Build crates/grave-wasm into viewer/pkg once wasm32-unknown-unknown, wasm-pack, and the decoder dependency are available.",
      true,
    );
    console.error(error);
  }
}

async function loadGrave(file) {
  if (!state.wasm) {
    setStatus(
      "This viewer is waiting on a wasm build. The grave can be selected, but it cannot yet be rendered in the browser on this machine.",
      true,
    );
    return;
  }

  setStatus(`Preparing ${file.name}...`);
  state.bytes = new Uint8Array(await file.arrayBuffer());
  state.headerInfo = await state.wasm.read_header(state.bytes);
  const todayTimestamp = endOfDay(state.todayDay);
  const initialRender = await state.wasm.render_at(state.bytes, todayTimestamp);

  configureTimeline(state.headerInfo.header, initialRender.prognosis);
  elements.recordTitle.textContent = state.headerInfo.header.original_filename || file.name;
  await renderSelectedDay(Number(elements.slider.value));
  setStatus(`${file.name} lies open behind the glass.`);
}

function configureTimeline(header, prognosis) {
  const buriedAt = toNumber(header.buried_at);
  const prognosisAt = toNumber(prognosis);
  const buriedDay = Math.floor(buriedAt / DAY_SECONDS);
  const prognosisSpan = Math.max(1, prognosisAt - buriedAt);
  const endTimestamp = prognosisAt + Math.ceil(prognosisSpan * 0.2);
  const endDay = Math.floor(endTimestamp / DAY_SECONDS);
  const selectedDay = clamp(state.todayDay, buriedDay, endDay);

  elements.slider.min = String(buriedDay);
  elements.slider.max = String(endDay);
  elements.slider.step = "1";
  elements.slider.value = String(selectedDay);
  elements.slider.disabled = false;

  elements.buriedLabel.textContent = `buried ${formatDate(buriedAt)}`;
  elements.terminalLabel.textContent = `horizon ${formatDate(prognosisAt)}`;

  const tickPosition = percentage(state.todayDay, buriedDay, endDay);
  elements.todayTick.style.left = `${tickPosition}%`;

  if (state.todayDay >= buriedDay && state.todayDay <= endDay) {
    const futureWidth = 100 - tickPosition;
    elements.futureRegion.style.width = `${futureWidth}%`;
  } else if (state.todayDay < buriedDay) {
    elements.futureRegion.style.width = "100%";
  } else {
    elements.futureRegion.style.width = "0%";
  }
}

async function renderSelectedDay(day) {
  const timestamp = endOfDay(day);
  const render = await state.wasm.render_at(state.bytes, timestamp);
  state.latestRender = render;

  elements.currentLabel.textContent = `selected ${formatDate(timestamp)}`;
  elements.scrubberLabel.textContent = `${formatDate(timestamp)} · ${render.q / 100}% decay`;
  updateFacts(render);
  showPayload(render);
}

function updateFacts(render) {
  const header = render.header;
  const buriedAt = toNumber(header.buried_at);
  const lastOpened = toNumber(header.last_opened);
  const ageDays = toNumber(render.age_days);
  const neglectDays = toNumber(render.neglect_days);
  const prognosisAt = toNumber(render.prognosis);
  const qBar = decayBar(render.q);
  const facts = [
    ["Interred", `${header.original_filename} (${header.mimetype})`],
    ["Buried", `${formatDate(buriedAt)} (${ageDays} days ago)`],
    ["Last visited", `${formatDate(lastOpened)} (${neglectDays} days ago)`],
    ["Visits", String(header.open_count)],
    ["Profile", header.profile],
    [
      "Decay",
      `${qBar} ${(render.intensity * 100).toFixed(1)}%${render.disturbed ? " · disturbed" : ""}`,
    ],
    ["Prognosis", `terminal by ${formatDate(prognosisAt)}`],
    ["Epitaph", header.epitaph ? `"${header.epitaph}"` : "none"],
  ];

  elements.facts.innerHTML = facts
    .map(
      ([label, value]) =>
        `<div><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd></div>`,
    )
    .join("");
}

function showPayload(render) {
  elements.emptyState.hidden = true;
  elements.image.hidden = true;
  elements.text.hidden = true;
  elements.headstone.hidden = true;

  if (render.payload.kind === "image") {
    drawImage(render.payload);
    return;
  }

  if (render.payload.kind === "text") {
    elements.text.textContent = render.payload.text;
    elements.text.dataset.hexDump = String(Boolean(render.payload.is_hex_dump));
    elements.text.hidden = false;
    return;
  }

  elements.headstone.innerHTML = headstoneSvg(render);
  elements.headstone.hidden = false;
}

function drawImage(payload) {
  const sourceCanvas = document.createElement("canvas");
  sourceCanvas.width = payload.width;
  sourceCanvas.height = payload.height;
  const sourceContext = sourceCanvas.getContext("2d");
  const imageData = new ImageData(
    new Uint8ClampedArray(payload.rgba),
    payload.width,
    payload.height,
  );
  sourceContext.putImageData(imageData, 0, 0);

  const scale = Math.min(1, MAX_PREVIEW_EDGE / Math.max(payload.width, payload.height));
  const width = Math.max(1, Math.round(payload.width * scale));
  const height = Math.max(1, Math.round(payload.height * scale));
  elements.image.width = width;
  elements.image.height = height;
  const context = elements.image.getContext("2d");
  context.clearRect(0, 0, width, height);
  context.drawImage(sourceCanvas, 0, 0, width, height);
  elements.image.hidden = false;
}

function headstoneSvg(render) {
  const header = render.header;
  const buriedAt = toNumber(header.buried_at);
  const prognosisAt = toNumber(render.prognosis);
  const openCount = toNumber(header.open_count);
  const mournCredit = toNumber(header.mourn_credit);
  const epitaphLines = wrapWords(header.epitaph ? `"${header.epitaph}"` : "", 18).slice(0, 4);
  const filenameLines = wrapWords(header.original_filename || "unknown", 16).slice(0, 2);
  const detail = `${visitPhrase(openCount)} · ${mournPhrase(mournCredit)}`;
  return `
    <svg viewBox="0 0 520 640" role="img" aria-label="Terminal headstone">
      <defs>
        <linearGradient id="stone" x1="0%" x2="0%" y1="0%" y2="100%">
          <stop offset="0%" stop-color="#d0c3b2"></stop>
          <stop offset="100%" stop-color="#7d6f60"></stop>
        </linearGradient>
      </defs>
      <rect width="520" height="640" fill="transparent"></rect>
      <ellipse cx="260" cy="564" rx="176" ry="34" fill="rgba(0,0,0,0.28)"></ellipse>
      <path d="M132 522V216c0-72 57-126 128-126s128 54 128 126v306z" fill="url(#stone)" stroke="#4a4038" stroke-width="6"></path>
      <path d="M176 522V236c0-51 37-90 84-90s84 39 84 90v286z" fill="rgba(255,255,255,0.08)"></path>
      <text x="260" y="160" text-anchor="middle" font-size="28" fill="#2e241d" font-family="Georgia, serif">✝ RIP</text>
      ${filenameLines.map((line, index) => `<text x="260" y="${236 + index * 34}" text-anchor="middle" font-size="25" fill="#2e241d" font-family="Georgia, serif">${escapeHtml(line)}</text>`).join("")}
      <text x="260" y="324" text-anchor="middle" font-size="20" fill="#2e241d" font-family="Georgia, serif">${formatYear(buriedAt)} - ${formatYear(prognosisAt)}</text>
      ${epitaphLines.map((line, index) => `<text x="260" y="${398 + index * 28}" text-anchor="middle" font-size="19" fill="#2e241d" font-family="Georgia, serif">${escapeHtml(line)}</text>`).join("")}
      <text x="260" y="570" text-anchor="middle" font-size="18" fill="#f2e7d8" font-family="'IBM Plex Mono', monospace">This file has reached terminal decomposition.</text>
      <text x="260" y="596" text-anchor="middle" font-size="15" fill="#c7b59e" font-family="'IBM Plex Mono', monospace">${escapeHtml(detail)}</text>
    </svg>
  `;
}

function decayBar(q) {
  const slots = 12;
  const filled = Math.min(slots, Math.ceil((q / 10_000) * slots));
  return "█".repeat(filled).padEnd(slots, "░");
}

function visitPhrase(openCount) {
  return openCount === 1 ? "1 visit" : `${openCount} visits`;
}

function mournPhrase(mournCredit) {
  if (mournCredit === 0) {
    return "never mourned";
  }
  if (mournCredit === 1) {
    return "mourned once";
  }
  if (mournCredit === 2) {
    return "mourned twice";
  }
  return `mourned ${mournCredit} times`;
}

function percentage(day, minDay, maxDay) {
  if (maxDay <= minDay) {
    return 0;
  }
  return ((day - minDay) / (maxDay - minDay)) * 100;
}

function endOfDay(day) {
  return BigInt(day * DAY_SECONDS + DAY_SECONDS - 1);
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function formatDate(timestamp) {
  return new Date(toNumber(timestamp) * 1000).toISOString().slice(0, 10);
}

function formatYear(timestamp) {
  return new Date(toNumber(timestamp) * 1000).getUTCFullYear();
}

function wrapWords(text, width) {
  if (!text) {
    return [];
  }
  const words = text.split(/\s+/);
  const lines = [];
  let current = "";
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    if (candidate.length > width && current) {
      lines.push(current);
      current = word;
    } else {
      current = candidate;
    }
  }
  if (current) {
    lines.push(current);
  }
  return lines;
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function setStatus(message, isError = false) {
  elements.status.textContent = message;
  elements.status.classList.toggle("is-error", isError);
}

function toNumber(value) {
  return typeof value === "bigint" ? Number(value) : value;
}
