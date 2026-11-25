import test from 'node:test';
import assert from 'node:assert/strict';
import {
  applyToneMark,
  composePinyin,
  getAudioUrls,
  getAvailableTones,
  getExampleChar,
} from './pinyinEngine';

test('composePinyin composes standard Mandarin syllables and rejects illegal combinations', () => {
  const ba = composePinyin('b', 'a');
  assert.equal(ba.valid, true);
  assert.equal(ba.syllable, 'ba');
  assert.equal(ba.isUmlautDrop, false);

  const bong = composePinyin('b', 'ong');
  assert.equal(bong.valid, false);
  assert.ok(bong.reason && bong.reason.includes('b'));

  const zeroA = composePinyin('', 'a');
  assert.equal(zeroA.valid, true);
  assert.equal(zeroA.syllable, 'a');
});

test('composePinyin enforces j/q/x/y + ü umlaut drop rule and n/l preservation', () => {
  const ju = composePinyin('j', 'ü');
  assert.equal(ju.valid, true);
  assert.equal(ju.syllable, 'ju');
  assert.equal(ju.isUmlautDrop, true);

  const qu = composePinyin('q', 'üe');
  assert.equal(qu.valid, true);
  assert.equal(qu.syllable, 'que');
  assert.equal(qu.isUmlautDrop, true);

  const lv = composePinyin('l', 'ü');
  assert.equal(lv.valid, true);
  assert.equal(lv.syllable, 'lü');
  assert.equal(lv.isUmlautDrop, false);

  const juBad = composePinyin('j', 'u');
  assert.equal(juBad.valid, false);
  assert.ok(juBad.reason && juBad.reason.includes('ü'));
});

test('applyToneMark follows official vowel priority rules', () => {
  assert.equal(applyToneMark('ma', 1), 'mā');
  assert.equal(applyToneMark('ma', 3), 'mǎ');
  assert.equal(applyToneMark('hao', 3), 'hǎo');
  assert.equal(applyToneMark('liu', 2), 'liú');
  assert.equal(applyToneMark('gui', 4), 'guì');
  assert.equal(applyToneMark('lü', 4), 'lǜ');
  assert.equal(applyToneMark('jue', 2), 'jué');
});

test('getAvailableTones and getAudioUrls match flashtony2005/pinyin layout', () => {
  const tones = getAvailableTones('ma');
  assert.deepEqual(tones, [1, 2, 3, 4]);

  const urls = getAudioUrls('nü', 3);
  assert.equal(urls.length, 2);
  assert.ok(urls[0].endsWith('/bd/n%C3%BC3.mp3'));
  assert.ok(urls[1].endsWith('/n%C3%BC.m4a'));

  const example = getExampleChar('ma', 3);
  assert.equal(example.char, '马');
});

test('composePinyin supports three-part syllables (三拼音节) like jia, hua, duo, and juan', () => {
  const jia = composePinyin('j', 'a', 'i');
  assert.equal(jia.valid, true);
  assert.equal(jia.syllable, 'jia');
  assert.equal(jia.isTriplet, true);
  assert.equal(applyToneMark(jia.syllable, 1), 'jiā');
  assert.equal(getExampleChar('jia', 1).char, '家');

  const juan = composePinyin('j', 'an', 'ü');
  assert.equal(juan.valid, true);
  assert.equal(juan.syllable, 'juan');
  assert.equal(juan.isUmlautDrop, true);
  assert.equal(applyToneMark(juan.syllable, 3), 'juǎn');

  const hua = composePinyin('h', 'a', 'u');
  assert.equal(hua.valid, true);
  assert.equal(hua.syllable, 'hua');
  assert.equal(applyToneMark(hua.syllable, 1), 'huā');

  // Helpful nudge when forgetting medial i for j + a
  const jaMissingMedial = composePinyin('j', 'a');
  assert.equal(jaMissingMedial.valid, false);
  assert.ok(jaMissingMedial.reason && jaMissingMedial.reason.includes('介母「i」'));

  // Triplet not in baidu repo audio returns empty remote URL list (no 404s)
  assert.deepEqual(getAudioUrls('jia', 1), []);
});
