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
