const { h: createElement, render } = window.preact;
const { useState, useEffect, useRef, useCallback } = window.preactHooks;
const html = window.htm.bind(createElement);
const DICT = window.WEB_I18N || { zh: {} };
let translate = (key) => key;
let LANG = "zh";
const LOCALES = { zh: "zh-CN", en: "en-US", ja: "ja-JP" };
function setLang(lang) {
  LANG = DICT[lang] ? lang : "zh";
  const dict = DICT[LANG];
  translate = (key) => (dict[key] != null ? dict[key] : (DICT.zh[key] != null ? DICT.zh[key] : key));
}

/* ---------- 工具 ---------- */
const LUCIDE = window.LUCIDE || {};
function Icon({ name, size = 18, cls = "" }) {
  const inner = LUCIDE[name] || "";
  return html`<svg class=${"ico " + cls} width=${size} height=${size} viewBox="0 0 24 24"
    fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
    dangerouslySetInnerHTML=${{ __html: inner }}></svg>`;
}
function humanSize(bytes) {
  if (bytes == null) return "-";
  const units = ["B", "KB", "MB", "GB"]; let index = 0, value = bytes;
  while (value >= 1024 && index < units.length - 1) { value /= 1024; index++; }
  return (index === 0 ? value : value.toFixed(1)) + " " + units[index];
}
function mmss(sec) {
  if (!sec && sec !== 0) return "0:00";
  const secondsPart = Math.floor(sec % 60), minutesPart = Math.floor(sec / 60);
  return minutesPart + ":" + String(secondsPart).padStart(2, "0");
}
function fmtDate(mtime) {
  try { return new Date(mtime * 1000).toLocaleString(LOCALES[LANG] || "zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }); }
  catch (error) { return ""; }
}
async function api(path, opts) {
  const response = await fetch(path, Object.assign({ headers: { "Content-Type": "application/json" } }, opts || {}));
  if (!response.ok) throw new Error("HTTP " + response.status);
  return response.json();
}
const fileUrl = (id) => "/api/file?id=" + encodeURIComponent(id);
const thumbUrl = (id) => "/api/thumb?id=" + encodeURIComponent(id);

/* ---------- 重试图片 ---------- */
export function Thumb({ id, kind }) {
  const [ok, setOk] = useState(true);
  if (!ok) return html`<div class="ph"><${Icon} name=${kind === "video" ? "film" : "image"} size=${30} /></div>`;
  return html`<img draggable="false" src=${thumbUrl(id)} loading="lazy" onError=${() => setOk(false)} alt="" />`;
}


export {
  createElement, render, useState, useEffect, useRef, useCallback, html,
  translate, setLang, LOCALES, Icon, humanSize, mmss, fmtDate, api, fileUrl, thumbUrl,
};
