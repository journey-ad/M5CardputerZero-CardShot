import { html, useState, useEffect, useRef, Icon, humanSize, mmss, fileUrl, translate } from "../core.js";

function Lightbox({ items, index, onClose, onNav, onDelete, zoom, setZoom, toast }) {
  const it = items[index];
  const stageRef = useRef(null);
  useEffect(() => {
    const onKeyDown = (event) => {
      if (event.key === "Escape") onClose();
      else if (event.key === "ArrowLeft") onNav(-1);
      else if (event.key === "ArrowRight") onNav(1);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [index]);
  if (!it) return null;
  return html`<div class="lightbox" onClick=${(event) => { if (event.target.classList.contains("lightbox") || event.target.classList.contains("lb-stage")) onClose(); }}>
    <div class="lb-bar">
      <${Icon} name=${it.kind === "video" ? "film" : "image"} size=${18} />
      <div class="lb-title">${it.name}</div>
      <div class="lb-meta">${humanSize(it.size)}${it.duration ? " · " + mmss(it.duration) : ""}</div>
      <div class="spacer"></div>
      <div class="lb-zoom">
        ${[1, 2, 3].map((level) => html`<button class=${zoom === level ? "active" : ""} onClick=${() => setZoom(level)}>${level}X</button>`)}
      </div>
      <button class="btn btn-ghost btn-sm" onClick=${onClose}><${Icon} name="x" size=${18} /></button>
    </div>
    <div class="lb-stage" ref=${stageRef}>
      ${index > 0 && html`<button class="lb-nav lb-prev" onClick=${() => onNav(-1)}><${Icon} name="chevron-left" /></button>`}
      ${it.kind === "video"
        ? html`<video src=${fileUrl(it.id)} controls autoplay loop playsinline draggable="false"
            style=${{ width: (320 * zoom) + "px", height: (170 * zoom) + "px", maxWidth: "none", maxHeight: "none", imageRendering: "pixelated" }}></video>`
        : html`<img src=${fileUrl(it.id)} alt="" draggable="false"
            style=${{ width: (320 * zoom) + "px", height: (170 * zoom) + "px", maxWidth: "none", maxHeight: "none", imageRendering: "pixelated" }} />`}
      ${index < items.length - 1 && html`<button class="lb-nav lb-next" onClick=${() => onNav(1)}><${Icon} name="chevron-right" /></button>`}
    </div>
    <div class="lb-actions">
      <button class="btn btn-secondary" onClick=${async () => {
        const el = stageRef.current && stageRef.current.querySelector("img,video");
        if (!el) return;
        const width = 320 * zoom, height = 170 * zoom;
        const cv = document.createElement("canvas"); cv.width = width; cv.height = height;
        const ctx = cv.getContext("2d"); ctx.imageSmoothingEnabled = false;
        try { ctx.drawImage(el, 0, 0, width, height); } catch (error) { toast && toast(translate("copy_failed"), true); return; }
        const dataUrl = cv.toDataURL("image/png");
        let done = false;
        if (navigator.clipboard && window.ClipboardItem) {
          try {
            const blob = await new Promise((res) => cv.toBlob(res, "image/png"));
            await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
            done = true;
          } catch (error) { done = false; }
        }
        if (!done) {
          const box = document.createElement("div");
          box.contentEditable = "true";
          box.style.cssText = "position:fixed;left:-9999px;top:0;opacity:0";
          const im = document.createElement("img");
          im.src = dataUrl;
          box.appendChild(im);
          document.body.appendChild(box);
          const range = document.createRange();
          range.selectNode(im);
          const sel = window.getSelection();
          sel.removeAllRanges(); sel.addRange(range);
          try { done = document.execCommand("copy"); } catch (error) { done = false; }
          sel.removeAllRanges(); box.remove();
        }
        if (done) { toast && toast(translate("copied")); return; }
        const link = document.createElement("a");
        link.href = dataUrl;
        link.download = (it.name || "capture").replace(/\.[^.]+$/, "") + "@" + zoom + "x.png";
        link.click();
        toast && toast(translate("copy_download"));
      }}><${Icon} name="copy" size=${16} />${translate("copy")}</button>
      <button class="btn btn-secondary" onClick=${() => {
        if (it.kind === "video") {
          const link = document.createElement("a");
          link.href = fileUrl(it.id) + "&download=1";
          link.download = it.name || "capture";
          link.click();
          return;
        }
        const el = stageRef.current && stageRef.current.querySelector("img");
        if (!el) return;
        const width = 320 * zoom, height = 170 * zoom;
        const cv = document.createElement("canvas"); cv.width = width; cv.height = height;
        const ctx = cv.getContext("2d"); ctx.imageSmoothingEnabled = false;
        try { ctx.drawImage(el, 0, 0, width, height); } catch (error) { toast && toast(translate("down_failed"), true); return; }
        const link = document.createElement("a");
        link.href = cv.toDataURL("image/png");
        link.download = (it.name || "capture").replace(/\.[^.]+$/, "") + "@" + zoom + "x.png";
        link.click();
      }}><${Icon} name="download" size=${16} />${translate("download")}</button>
      <button class="btn btn-destructive" onClick=${() => onDelete(it.id)}><${Icon} name="trash-2" size=${16} />${translate("delete")}</button>
    </div>
  </div>`;
}

export { Lightbox };
