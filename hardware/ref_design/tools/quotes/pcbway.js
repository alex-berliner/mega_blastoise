// PCBWay bare-board quote from the form alone (no files uploaded).
// usage: node pcbway.js <layersText> <qty> <holeText> <shot.png>
const { chromium } = require("playwright-core");
const [layers, qty, hole, shot] = process.argv.slice(2);
async function rowClick(page, label, text) {
  const pt = await page.evaluate(([label, text]) => {
    const vis = e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== "hidden"; };
    const own = e => [...e.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join("").trim();
    const all = [...document.querySelectorAll("body *")];
    const lab = all.find(e => vis(e) && own(e).startsWith(label));
    if (!lab) return { err: "no label" };
    lab.scrollIntoView({ block: "center" });
    const lr = lab.getBoundingClientRect();
    let best = null;
    for (const e of all) {
      if (!vis(e) || e.innerText == null) continue;
      const t = e.innerText.trim();
      if (!(t === text || (t.startsWith(text) && t.length <= text.length + 3))) continue;
      const r = e.getBoundingClientRect();
      const dy = Math.abs(r.top + r.height / 2 - (lr.top + lr.height / 2));
      if (dy < 30 && r.left > lr.left + 60 && (!best || r.width * r.height < best.a)) best = { x: r.left + r.width / 2, y: r.top + r.height / 2, a: r.width * r.height };
    }
    return best || { err: "no option" };
  }, [label, text]);
  if (pt.err) throw new Error(`${pt.err}: "${text}" in row "${label}"`);
  await page.mouse.click(pt.x, pt.y);
  await page.waitForTimeout(1000);
}
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME || process.env.HOME + "/.cache/ms-playwright/chromium-1229/chrome-linux64/chrome", headless: true });
  const page = await browser.newPage({ viewport: { width: 1400, height: 1800 }, userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" });
  await page.goto("https://www.pcbway.com/orderonline.aspx", { waitUntil: "domcontentloaded", timeout: 60000 });
  await page.waitForTimeout(6000);
  await page.fill("#hidLength", "151.34");
  await page.fill("#hidWidth", "70.5");
  await page.click("#hidNum").catch(() => {});
  await page.waitForTimeout(800);
  const q = page.locator(`input[name=countNumer][value="${qty}"]`);
  if (await q.count()) { await q.first().check({ force: true }).catch(async () => { await q.first().click({ force: true }); }); }
  else { await page.fill("#hidNum", qty); }
  await page.waitForTimeout(800);
  await rowClick(page, "Layers", layers);
  await rowClick(page, "Min hole size", hole);
  await rowClick(page, "Surface finish", "Immersion gold(ENIG)");
  await page.waitForTimeout(1500);
  const calc = page.locator("input[value=Calculate], a, button, div").filter({ hasText: /^\s*Calculate\s*$/ });
  await page.locator("input[type=button][value=Calculate]").first().click({ force: true }).catch(async () => { await calc.first().click(); });
  await page.waitForTimeout(6000);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(800);
  await page.screenshot({ path: shot, fullPage: false });
  const qtyShown = await page.inputValue("#hidNum").catch(() => "?");
  console.log(`== PCBWay ${layers}, qty field ${qtyShown}, hole ${hole}, ENIG`);
  const body = await page.evaluate(() => document.body.innerText);
  const lines = body.split("\n").map(s => s.trim()).filter(Boolean);
  const k = lines.findIndex(l => /Build Time|Price|Total/i.test(l));
  console.log(lines.filter(l => /\$\s?\d/.test(l)).slice(0, 30).join("\n"));
  await browser.close();
})().catch(e => { console.error("ERR", e.message); process.exit(1); });
