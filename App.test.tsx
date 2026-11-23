import { expect, open, test } from "nsh/test";

test("renders initials, finals and practises ba", async () => {
  const app = await open(".");

  const texts = app.texts();
  expect(texts).toContain("拼音拼读");
  expect(texts).toContain("声母");
  expect(texts).toContain("韵母");
  expect(texts).toContain("b");
  expect(texts).toContain("zh");
  expect(texts).toContain("a");
  expect(texts).toContain("ü");

  // Tap an initial, then a final: the composed syllable appears.
  await app.press("id:tile-b");
  await app.press("id:tile-a");
  await app.waitFor("text:ba");

  // An illegal combination (b + e) must not change the result.
  await app.press("id:tile-e");
  expect(app.texts()).toContain("ba");

  // Four tone buttons exist and switching tone keeps the composed result.
  for (const tone of ["ā", "á", "ǎ", "à"]) {
    expect(texts).toContain(tone);
  }
  await app.press("id:tone-3");
  expect(app.texts()).toContain("ba");

  // Switch to the syllable table.
  await app.press("text:音节表");
  expect(app.texts()).toContain("音节表");
});
