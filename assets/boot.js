// The tile tilts toward the point you press, the way Metro tiles did.
(() => {
  if (window.__tabscribe) return;
  window.__tabscribe = true;
  document.addEventListener("pointerdown", (e) => {
    const t = e.target.closest(".tilt");
    if (!t || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const r = t.getBoundingClientRect();
    const dx = (e.clientX - r.left) / r.width - 0.5, dy = (e.clientY - r.top) / r.height - 0.5;
    t.style.transform = `perspective(900px) rotateX(${-dy * 6}deg) rotateY(${dx * 6}deg) scale(.985)`;
    const up = () => { t.style.transform = ""; removeEventListener("pointerup", up); removeEventListener("pointercancel", up); };
    addEventListener("pointerup", up);
    addEventListener("pointercancel", up);
  });
})();
