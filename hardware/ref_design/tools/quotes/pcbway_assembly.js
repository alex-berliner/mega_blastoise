// PCBWay turnkey assembly labor quote (no files uploaded). Parts and PCB are priced separately.
// usage: node pwa.js <qty> <unique> <smd> <bga_qfp> <tht>
const { chromium } = require("playwright-core");
const [qty, uniq, smd, bga, tht] = process.argv.slice(2);
async function fillRow(page, keyword, value) {
  const pt = await page.evaluate(kw => {
    const vis = e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; };
    const labs = [...document.querySelectorAll("body *")].filter(e => vis(e) && e.children.length <= 2 && e.innerText && e.innerText.replace(/\s+/g, " ").includes(kw) && e.innerText.length < 60);
    if (!labs.length) return { err: "no label " + kw };
    labs.sort((a, b) => a.innerText.length - b.innerText.length);
    const lab = labs[0]; lab.scrollIntoView({ block: "center" });
    const lr = lab.getBoundingClientRect();
    const ins = [...document.querySelectorAll("input[type=text]")].filter(vis).map(i => ({ i, r: i.getBoundingClientRect() }))
      .filter(o => Math.abs(o.r.top + o.r.height / 2 - (lr.top + lr.height / 2)) < 30 && o.r.left > lr.left + 40);
    if (!ins.length) return { err: "no input " + kw };
    ins.sort((a, b) => a.r.left - b.r.left);
    const r = ins[0].r; return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  }, keyword);
  if (pt.err) throw new Error(pt.err);
  await page.mouse.click(pt.x, pt.y);
  await page.keyboard.press("Control+A");
  await page.keyboard.type(String(value));
  await page.keyboard.press("Tab");
  await page.waitForTimeout(500);
}
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME || process.env.HOME + "/.cache/ms-playwright/chromium-1229/chrome-linux64/chrome", headless: true });
  const page = await browser.newPage({ viewport: { width: 1400, height: 1400 }, userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" });
  await page.goto("https://www.pcbway.com/quotesmt.aspx", { waitUntil: "domcontentloaded", timeout: 60000 });
  await page.waitForTimeout(6000);
  await fillRow(page, "Quantity", qty);
  await page.getByText("Other Parameters", { exact: false }).first().click();
  await page.waitForTimeout(1500);
  await fillRow(page, "Unique Parts", uniq);
  await fillRow(page, "SMD Parts", smd);
  await fillRow(page, "BGA/QFP Parts", bga);
  await fillRow(page, "Through-Hole Parts", tht);
  await page.locator("input[type=button][value=Calculate]").first().click({ force: true });
  await page.waitForTimeout(6000);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(800);
  await page.screenshot({ path: "pwa_q.png", fullPage: false });
  const body = await page.evaluate(() => document.body.innerText);
  const i = body.indexOf("Pricing And Build Time");
  console.log(`== PCBWay assembly: qty ${qty}, unique ${uniq}, SMD ${smd}, BGA/QFP ${bga}, THT ${tht}`);
  console.log(i >= 0 ? body.slice(i, i + 700).replace(/\n{2,}/g, "\n") : body.split("\n").filter(l => /\$\s?\d/.test(l)).slice(0, 20).join("\n"));
  await browser.close();
})().catch(e => { console.error("ERR", e.message); process.exit(1); });
