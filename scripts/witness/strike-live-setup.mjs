import { playUrl } from "./play-url.mjs";
async function clickWhenEnabled(page, text, timeout) {
  await page.waitForFunction((label) => {
    const button = [...document.querySelectorAll("button.flow-action")].find((b) => b.textContent.trim() === label);
    return button !== undefined && !button.disabled;
  }, text, { timeout });
  await page.evaluate((label) => {
    [...document.querySelectorAll("button.flow-action")].find((b) => b.textContent.trim() === label).click();
  }, text);
  return text;
}

export async function livePhase(page) {
  return page.evaluate(() => document.body.dataset.phase ?? "");
}

export async function setupLiveMatch(page, ns, { timeout = 180000 } = {}) {
  const steps = {};
  await page.goto(playUrl({ host: 1, ns }, "#match-" + ns), { waitUntil: "domcontentloaded", timeout: 50000 });
  await page.waitForFunction(() => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function", null, { timeout });
  steps.mode = await page.evaluate(() => window.oscraft.lobby("mode", { code: "ap" }));
  steps.claim = await page.evaluate(() => window.oscraft.lobby("claim", { slot: 0 }));
  await page.waitForFunction(() => [...document.querySelectorAll(".flow-hero")].some((e) => e.dataset.acro === "PUD"), null, { timeout: 30000 });
  steps.hero = await page.evaluate(() => { [...document.querySelectorAll(".flow-hero")].find((e) => e.dataset.acro === "PUD").click(); return "PUD"; });
  await page.waitForFunction(() => document.getElementById("gui-flow")?.dataset.stage === "podium", null, { timeout: 30000 });
  steps.fill = await clickWhenEnabled(page, "Fill with bots", 30000);
  steps.ready = await clickWhenEnabled(page, "Ready", 30000);
  steps.start = await clickWhenEnabled(page, "Start match", 120000);
  await page.waitForFunction(() => document.body.dataset.phase === "live", null, { timeout });
  steps.phase = await livePhase(page);
  if (steps.phase !== "live") throw new Error("strike setup did not reach a live match: phase is " + steps.phase);
  return steps;
}
