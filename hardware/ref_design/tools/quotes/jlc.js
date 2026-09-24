// JLC bare-board quote from the form alone (no files uploaded).
// usage: node jlc.js <layers> <qty> <finish> <via> <shot.png>
const { chromium } = require("playwright-core");
const [layers, qty, finish, via, shot] = process.argv.slice(2);
// click the option whose text is `text` on the same row as the visible label `label`
async function rowClick(page, label, text) {
  for (let pass = 0; pass < 2; pass++) {
    const pt = await page.evaluate(([label, text]) => {
      const vis = e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== "hidden"; };
      const all = [...document.querySelectorAll("body *")];
      const own = e => [...e.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join("").trim();
      const labs = all.filter(e => vis(e) && own(e) === label);
      if (!labs.length) return { err: "no label" };
      const lab = labs[0];
      lab.scrollIntoView({ block: "center" });
      const lr = lab.getBoundingClientRect();
      let best = null;
      for (const e of all) {
        if (!vis(e) || e.innerText == null || e.innerText.trim() !== text) continue;
        const r = e.getBoundingClientRect();
        const dy = Math.abs(r.top + r.height / 2 - (lr.top + lr.height / 2));
        if (dy < 25 && r.left > lr.left + 50 && (!best || r.width * r.height < best.a)) best = { x: r.left + r.width / 2, y: r.top + r.height / 2, a: r.width * r.height };
      }
      return best || { err: "no option" };
    }, [label, text]);
    if (!pt.err) { await page.mouse.click(pt.x, pt.y); await page.waitForTimeout(1500); return pt; }
    if (pass) throw new Error(`${pt.err}: "${text}" in row "${label}"`);
    await page.waitForTimeout(3000);
  }
}
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME || process.env.HOME + "/.cache/ms-playwright/chromium-1229/chrome-linux64/chrome", headless: true });
  const page = await browser.newPage({ viewport: { width: 1400, height: 1800 }, userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" });
  await page.goto("https://cart.jlcpcb.com/quote", { waitUntil: "domcontentloaded", timeout: 60000 });
  await page.waitForTimeout(8000);
  await rowClick(page, "Layers", layers);
  const dims = page.locator("input[name=stencilLength], input[name=stencilWidth]");
  await dims.nth(0).fill("70.5"); await dims.nth(0).press("Tab");
  await dims.nth(1).fill("151.34"); await dims.nth(1).press("Tab");
  await page.waitForTimeout(1500);
  const anchor = await rowClick(page, "PCB Qty", "5");
  const pick = await page.evaluate(([q, ay]) => {
    const els = [...document.querySelectorAll("button, li, div, span")].filter(e => {
      const r = e.getBoundingClientRect();
      return r.width > 0 && r.height > 0 && e.innerText && e.innerText.trim() === q && r.top > ay;
    });
    els.sort((a, b) => a.getBoundingClientRect().top - b.getBoundingClientRect().top ||
      a.getBoundingClientRect().width * a.getBoundingClientRect().height - b.getBoundingClientRect().width * b.getBoundingClientRect().height);
    const r = els.length && els[0].getBoundingClientRect();
    return r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : null;
  }, [qty, anchor.y + 5]);
  if (!pick) throw new Error("no qty " + qty);
  await page.mouse.click(pick.x, pick.y);
  await page.waitForTimeout(1500);
  await rowClick(page, "Surface Finish", finish);
  if (via !== "default") await rowClick(page, "Min via hole size/diameter", via);
  await page.waitForTimeout(3000);
  await page.screenshot({ path: shot, fullPage: false });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(800);
  await page.screenshot({ path: shot.replace(".png", "_top.png"), fullPage: false });
  const body = await page.evaluate(() => document.body.innerText);
  const i = body.indexOf("Charge Details"), j = body.indexOf("Coupons", i);
  console.log(`== ${layers} layers, ${qty} pcs, ${finish}, via ${via}`);
  console.log(body.slice(i, j > i ? j + 60 : i + 900).replace(/\n{2,}/g, "\n"));
  await browser.close();
})().catch(e => { console.error("ERR", e.message); process.exit(1); });
