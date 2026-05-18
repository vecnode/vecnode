// --- health check (existing) ---
const statusNode = document.getElementById("status");
const button = document.getElementById("check");

async function checkHealth() {
  statusNode.textContent = "Checking API...";
  try {
    const response = await fetch("http://localhost:8086/health");
    if (!response.ok) throw new Error("HTTP " + response.status);
    const data = await response.json();
    statusNode.textContent = "API is healthy at " + data.time;
  } catch (error) {
    statusNode.textContent = "API check failed: " + error.message;
  }
}

button.addEventListener("click", checkHealth);
checkHealth();

// --- right panel drop zone (new) ---
const dropZone = document.getElementById("dropZone");
const pathListNode = document.getElementById("pathList");
const pickFilesBtn = document.getElementById("pickFilesBtn");
const pickFolderBtn = document.getElementById("pickFolderBtn");
const clearPathsBtn = document.getElementById("clearPathsBtn");
const pickFilesInput = document.getElementById("pickFilesInput");
const pickFolderInput = document.getElementById("pickFolderInput");

const collectedPathSet = new Set();
const collectedFileMap = new Map();

function isMarkdownPath(pathValue) {
  return /\.(md|markdown)$/i.test(String(pathValue || ""));
}

function normalizePath(pathValue) {
  return String(pathValue || "")
    .replace(/\\\\/g, "/")
    .replace(/^\/+/, "")
    .trim();
}

function addPath(pathValue) {
  const normalized = normalizePath(pathValue);
  if (!normalized) return;
  collectedPathSet.add(normalized);
}

function addCollectedFile(pathValue, file) {
  const normalized = normalizePath(pathValue);
  if (!normalized || !file) return;
  collectedFileMap.set(normalized, file);
  addPath(normalized);
}

function renderPathList(message) {
  if (message) {
    pathListNode.textContent = message;
    return;
  }

  const values = Array.from(collectedPathSet).sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" })
  );
  pathListNode.textContent = values.length
    ? values.join("\n")
    : "No files selected yet.";
}

function addFilesFromList(fileList) {
  for (const file of Array.from(fileList || [])) {
    const pathValue = file.webkitRelativePath || file.name;
    addCollectedFile(pathValue, file);
  }
  renderPathList();
}

function walkFileSystemEntry(entry, basePath = "") {
  return new Promise((resolve) => {
    if (!entry) {
      resolve();
      return;
    }

    const safeName = normalizePath(entry.name || "");
    const nextBase = basePath ? `${basePath}/${safeName}` : safeName;

    if (entry.isFile) {
      entry.file(
        (file) => {
          addCollectedFile(nextBase, file);
          resolve();
        },
        () => {
          resolve();
        }
      );
      return;
    }

    if (entry.isDirectory) {
      const reader = entry.createReader();
      const readBatch = () => {
        reader.readEntries(async (entries) => {
          if (!entries.length) {
            resolve();
            return;
          }
          for (const child of entries) {
            await walkFileSystemEntry(child, nextBase);
          }
          readBatch();
        }, resolve);
      };
      readBatch();
      return;
    }

    resolve();
  });
}

async function handleDrop(event) {
  event.preventDefault();
  dropZone.classList.remove("active");

  const dt = event.dataTransfer;
  if (!dt) {
    renderPathList("Drop failed: no data transfer.");
    return;
  }

  const items = Array.from(dt.items || []);
  const supportsEntries = items.some((item) => typeof item.webkitGetAsEntry === "function");

  if (supportsEntries) {
    for (const item of items) {
      const entry = typeof item.webkitGetAsEntry === "function" ? item.webkitGetAsEntry() : null;
      if (entry) {
        await walkFileSystemEntry(entry);
      }
    }
    renderPathList();
    return;
  }

  // Fallback when directory entries are not available in the browser.
  addFilesFromList(dt.files);
}

function setDropZoneActive(isActive) {
  dropZone.classList.toggle("active", isActive);
}

dropZone.addEventListener("dragenter", (event) => {
  event.preventDefault();
  setDropZoneActive(true);
});

dropZone.addEventListener("dragover", (event) => {
  event.preventDefault();
  setDropZoneActive(true);
});

dropZone.addEventListener("dragleave", (event) => {
  if (!dropZone.contains(event.relatedTarget)) {
    setDropZoneActive(false);
  }
});

dropZone.addEventListener("drop", handleDrop);

dropZone.addEventListener("click", () => {
  pickFilesInput.click();
});

dropZone.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    pickFilesInput.click();
  }
});

pickFilesBtn.addEventListener("click", () => {
  pickFilesInput.click();
});

pickFolderBtn.addEventListener("click", () => {
  pickFolderInput.click();
});

clearPathsBtn.addEventListener("click", () => {
  collectedPathSet.clear();
  collectedFileMap.clear();
  renderPathList();
});

pickFilesInput.addEventListener("change", () => {
  addFilesFromList(pickFilesInput.files);
  pickFilesInput.value = "";
});

pickFolderInput.addEventListener("change", () => {
  addFilesFromList(pickFolderInput.files);
  pickFolderInput.value = "";
});

// --- top menu tab switching ---
const menuButtons = document.querySelectorAll(".menu-btn");
const processorPanels = document.querySelectorAll(".processor-panel");

function showPanel(panelId) {
  for (const panel of processorPanels) {
    panel.classList.remove("active");
  }
  const nextPanel = document.getElementById(panelId);
  if (nextPanel) {
    nextPanel.classList.add("active");
  }

  for (const btn of menuButtons) {
    const isActive = btn.dataset.target === panelId;
    btn.classList.toggle("active", isActive);
    btn.setAttribute("aria-selected", isActive ? "true" : "false");
  }
}

for (const btn of menuButtons) {
  btn.addEventListener("click", () => {
    showPanel(btn.dataset.target);
  });
}

// --- pandoc processor (new) ---
const pandocVersionBtn = document.getElementById("pandocVersionBtn");
const pandocVersionStatus = document.getElementById("pandocVersionStatus");
const pandocMdToPdfBtn = document.getElementById("pandocMdToPdfBtn");
const pandocMdToPdf2Btn = document.getElementById("pandocMdToPdf2Btn");
const pandocConvertStatus = document.getElementById("pandocConvertStatus");

function formatSeconds(value) {
  return `${Math.max(0, value).toFixed(1)}s`;
}

pandocVersionBtn.addEventListener("click", async () => {
  pandocVersionStatus.textContent = "Loading pandoc version...";
  try {
    const res = await fetch("http://localhost:8086/pandoc/version");
    if (!res.ok) throw new Error("HTTP " + res.status);
    const data = await res.json();
    pandocVersionStatus.textContent = data.version || "unavailable";
  } catch (error) {
    pandocVersionStatus.textContent = "Error: " + error.message;
  }
});

pandocMdToPdfBtn.addEventListener("click", async () => {
  await runMarkdownToPdfConversion({
    endpoint: "http://localhost:8086/pandoc/markdown-to-pdf",
    modeLabel: "LaTeX-style",
  });
});

pandocMdToPdf2Btn.addEventListener("click", async () => {
  await runMarkdownToPdfConversion({
    endpoint: "http://localhost:8086/pandoc/markdown-to-pdf-viewer",
    modeLabel: "Viewer-style",
  });
});

async function runMarkdownToPdfConversion({ endpoint, modeLabel }) {
  const markdownEntries = Array.from(collectedFileMap.entries()).filter(([pathValue]) =>
    isMarkdownPath(pathValue)
  );

  if (!markdownEntries.length) {
    pandocConvertStatus.textContent = "No Markdown files found in the current paths.";
    return;
  }

  pandocConvertStatus.textContent =
    `Preparing ${markdownEntries.length} Markdown file(s)...\n` +
    `Mode: ${modeLabel}`;

  const form = new FormData();
  for (const [pathValue, file] of markdownEntries) {
    form.append("files", file, pathValue);
    form.append("paths", pathValue);
  }

  pandocConvertStatus.textContent =
    `Uploading ${markdownEntries.length} Markdown file(s) to API...\n` +
    `Mode: ${modeLabel}`;

  const requestStartedAt = performance.now();
  const progressTimer = window.setInterval(() => {
    const elapsed = (performance.now() - requestStartedAt) / 1000;
    pandocConvertStatus.textContent =
      `Converting ${markdownEntries.length} Markdown file(s)...\n` +
      `Mode: ${modeLabel}\n` +
      `Elapsed: ${formatSeconds(elapsed)}`;
  }, 700);

  try {
    const res = await fetch(endpoint, {
      method: "POST",
      body: form,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      throw new Error(err.detail || res.statusText);
    }

    const data = await res.json();
    const lines = [
      `Mode: ${modeLabel}`,
      `Output folder: ${data.output_folder}`,
      `Converted: ${data.converted_count}`,
      `Engine: ${data.engine || "pandoc"}`,
      `Time: ${formatSeconds(Number(data.duration_seconds || 0))}`,
    ];

    if (Array.isArray(data.converted_files) && data.converted_files.length) {
      lines.push("", "PDF files:", ...data.converted_files);
    }

    if (data.note) {
      lines.push("", `Note: ${data.note}`);
    }

    pandocConvertStatus.textContent = lines.join("\n");
  } catch (error) {
    pandocConvertStatus.textContent = "Error: " + error.message;
  } finally {
    window.clearInterval(progressTimer);
  }
}

// --- image upload (new) ---
const imageInput = document.getElementById("imageInput");
const uploadBtn = document.getElementById("uploadBtn");
const uploadStatus = document.getElementById("uploadStatus");
const results = document.getElementById("results");
const metaTableBody = document.querySelector("#metaTable tbody");
const previewOriginal = document.getElementById("previewOriginal");
const previewGray = document.getElementById("previewGray");

imageInput.addEventListener("change", () => {
  results.style.display = "none";
  uploadStatus.textContent = "";
});

uploadBtn.addEventListener("click", async () => {
  const file = imageInput.files[0];
  if (!file) {
    uploadStatus.textContent = "Please select an image first.";
    return;
  }

  uploadStatus.textContent = "Uploading...";
  results.style.display = "none";

  const form = new FormData();
  form.append("file", file);

  try {
    const res = await fetch("http://localhost:8086/process", {
      method: "POST",
      body: form,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      throw new Error(err.detail || res.statusText);
    }
    const data = await res.json();

    // Populate metadata table.
    metaTableBody.innerHTML = "";
    const rows = [
      ["Filename", data.filename],
      ["Format", data.format],
      ["Mode", data.mode],
      ["Dimensions", `${data.width} x ${data.height} px`],
      ["Size", (data.size_bytes / 1024).toFixed(1) + " KB"],
    ];
    for (const [label, value] of rows) {
      const tr = document.createElement("tr");
      const labelCell = document.createElement("td");
      labelCell.textContent = label;
      const valueCell = document.createElement("td");
      valueCell.textContent = value;
      tr.appendChild(labelCell);
      tr.appendChild(valueCell);
      metaTableBody.appendChild(tr);
    }

    // Show original preview.
    previewOriginal.src = URL.createObjectURL(file);

    // Show grayscale from base64.
    previewGray.src = "data:image/png;base64," + data.grayscale_png_b64;

    uploadStatus.textContent = "Done.";
    results.style.display = "block";
  } catch (error) {
    uploadStatus.textContent = "Error: " + error.message;
  }
});
