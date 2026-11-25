import {
  BAIDU_TONES,
  FINAL_MNEMONICS,
  INITIAL_MNEMONICS,
  PINYIN_TABLE,
  SPEAK_MAP,
  TONE_CHAR_MAP,
  TRIPLETS,
  WHOLE_SYLLABLES,
  type MedialLetter,
  type ToneNumber,
} from '../data/pinyinResources';

// Offline-first: the nsh shell ships the same recordings under /audio, so the
// game works without network. Point this back at the GitHub raw URL to use the
// upstream assets during web development.
export const REPO_AUDIO_BASE = '/audio';

export const TOTAL_DISCOVERABLE_SYLLABLES = (() => {
  const unique = new Set<string>(PINYIN_TABLE.syllables);
  for (const [ini, med, fin] of TRIPLETS) {
    const writtenMed = ['j', 'q', 'x'].includes(ini) && med === 'ü' ? 'u' : med;
    unique.add(`${ini}${writtenMed}${fin}`);
  }
  return unique.size;
})();

export interface ComposeOutcome {
  valid: boolean;
  syllable: string;
  initial: string;
  medial: MedialLetter;
  final: string;
  isTriplet: boolean;
  isUmlautDrop: boolean;
  isWholeSyllable: boolean;
  ruleNote: string;
  reason?: string;
}

const TONE_VOWEL_MAP: Record<string, [string, string, string, string]> = {
  a: ['ā', 'á', 'ǎ', 'à'],
  o: ['ō', 'ó', 'ǒ', 'ò'],
  e: ['ē', 'é', 'ě', 'è'],
  i: ['ī', 'í', 'ǐ', 'ì'],
  u: ['ū', 'ú', 'ǔ', 'ù'],
  ü: ['ǖ', 'ǘ', 'ǚ', 'ǜ'],
};

export const TONE_META: Record<
  ToneNumber,
  { name: string; contour: string; short: string; colorHex: string }
> = {
  1: { name: '一声 · 阴平', contour: '平直高扬 (55)', short: '一声平', colorHex: '#2A7B88' },
  2: { name: '二声 · 阳平', contour: '由中升高 (35)', short: '二声扬', colorHex: '#2E7D52' },
  3: { name: '三声 · 上声', contour: '先降后升 (214)', short: '三声拐', colorHex: '#B87B14' },
  4: { name: '四声 · 去声', contour: '迅速下降 (51)', short: '四声降', colorHex: '#B8473E' },
};

/**
 * Pure Mandarin syllable composer supporting both two-part (声母+韵母 / 零声母)
 * and three-part syllables (三拼音节: 声母 + 介母 i/u/ü + 韵母, e.g. j-i-a → jia).
 */
export function composePinyin(
  initial: string,
  final_: string,
  medial: MedialLetter = ''
): ComposeOutcome {
  const cleanInitial = initial.trim();
  const cleanFinal = final_.trim();
  const cleanMedial = (medial || '').trim() as MedialLetter;

  if (!cleanFinal) {
    return {
      valid: false,
      syllable: '',
      initial: cleanInitial,
      medial: cleanMedial,
      final: cleanFinal,
      isTriplet: cleanMedial !== '',
      isUmlautDrop: false,
      isWholeSyllable: false,
      ruleNote: '',
      reason: '请先放入一株韵母灵花作为药水基底。',
    };
  }

  // Three-part syllable mode (三拼音节: 声母 + 介母 + 韵母)
  if (cleanMedial !== '') {
    if (!cleanInitial) {
      return {
        valid: false,
        syllable: '',
        initial: '',
        medial: cleanMedial,
        final: cleanFinal,
        isTriplet: true,
        isUmlautDrop: false,
        isWholeSyllable: false,
        ruleNote: '',
        reason: '三拼音节必须先放入声母（如 j + i + a → jia，或 d + u + o → duo）。',
      };
    }

    const match = TRIPLETS.some(
      ([i, m, f]) => i === cleanInitial && m === cleanMedial && f === cleanFinal
    );

    if (!match) {
      const legalForPair = TRIPLETS.filter(
        ([i, m]) => i === cleanInitial && m === cleanMedial
      ).map(([, , f]) => f);

      let hint = `声母「${cleanInitial}」+ 介母「${cleanMedial}」不能和韵母「${cleanFinal}」组成三拼音节。`;
      if (['j', 'q', 'x'].includes(cleanInitial) && cleanMedial === 'u') {
        hint = `小秘密：j、q、x 只能搭配介母「i」或「ü」（如 ${cleanInitial}+i+a → ${cleanInitial}ia，${cleanInitial}+ü+an → ${cleanInitial}uan）！`;
      } else if (legalForPair.length > 0) {
        hint += `试试搭配后韵：${legalForPair.slice(0, 4).join('、')}！`;
      }
      return {
        valid: false,
        syllable: '',
        initial: cleanInitial,
        medial: cleanMedial,
        final: cleanFinal,
        isTriplet: true,
        isUmlautDrop: false,
        isWholeSyllable: false,
        ruleNote: '',
        reason: hint,
      };
    }

    const isUmlautDrop = ['j', 'q', 'x'].includes(cleanInitial) && cleanMedial === 'ü';
    const writtenMedial = isUmlautDrop ? 'u' : cleanMedial;
    const syllable = `${cleanInitial}${writtenMedial}${cleanFinal}`;
    const ruleNote = isUmlautDrop
      ? `触发「三拼 + ü 去点魔法」：${cleanInitial} + ${cleanMedial} + ${cleanFinal} → ${syllable}（声轻介快韵母响）！`
      : `触发「三拼音节」：${cleanInitial} + ${cleanMedial} + ${cleanFinal} → ${syllable}（声轻介快韵母响，连成一个音）！`;

    return {
      valid: true,
      syllable,
      initial: cleanInitial,
      medial: cleanMedial,
      final: cleanFinal,
      isTriplet: true,
      isUmlautDrop,
      isWholeSyllable: false,
      ruleNote,
    };
  }

  // Zero-initial standalone syllable (e.g. a, ai, ao, er, an, ang)
  if (!cleanInitial) {
    const legalZero = PINYIN_TABLE.zeroSyllables.includes(cleanFinal);
    if (!legalZero) {
      return {
        valid: false,
        syllable: '',
        initial: '',
        medial: '',
        final: cleanFinal,
        isTriplet: false,
        isUmlautDrop: false,
        isWholeSyllable: false,
        ruleNote: '',
        reason: `韵母「${cleanFinal}」无法单独成音，需要搭配声母。`,
      };
    }
    return {
      valid: true,
      syllable: cleanFinal,
      initial: '',
      medial: '',
      final: cleanFinal,
      isTriplet: false,
      isUmlautDrop: false,
      isWholeSyllable: WHOLE_SYLLABLES.has(cleanFinal),
      ruleNote: `零声母自成音节：${cleanFinal} 可独立发音！`,
    };
  }

  const legalFinals = PINYIN_TABLE.matrix[cleanInitial] ?? [];
  if (!legalFinals.includes(cleanFinal)) {
    const sampleFinals = legalFinals.slice(0, 4).join('、');
    let hint = `声母「${cleanInitial}」不能和韵母「${cleanFinal}」直接相拼。`;
    // Helpful nudge toward three-part syllables!
    const tripletViaI = TRIPLETS.some(
      ([i, m, f]) => i === cleanInitial && m === 'i' && f === cleanFinal
    );
    const tripletViaU = TRIPLETS.some(
      ([i, m, f]) => i === cleanInitial && m === 'u' && f === cleanFinal
    );
    if (tripletViaI) {
      hint = `「${cleanInitial}」和「${cleanFinal}」需要加上介母「i」做三拼音节哦：试试 ${cleanInitial} + i + ${cleanFinal} → ${cleanInitial}i${cleanFinal}！`;
    } else if (tripletViaU) {
      hint = `「${cleanInitial}」和「${cleanFinal}」需要加上介母「u」做三拼音节哦：试试 ${cleanInitial} + u + ${cleanFinal} → ${cleanInitial}u${cleanFinal}！`;
    } else if (['j', 'q', 'x'].includes(cleanInitial) && cleanFinal === 'u') {
      hint = `小秘密：j、q、x 从不和 u 相拼，它们喜欢和小鱼「ü」组合（脱帽写成 ${cleanInitial}u）！`;
    } else if (sampleFinals) {
      hint += `试试搭配 ${sampleFinals} 等韵母吧！`;
    }
    return {
      valid: false,
      syllable: '',
      initial: cleanInitial,
      medial: '',
      final: cleanFinal,
      isTriplet: false,
      isUmlautDrop: false,
      isWholeSyllable: false,
      ruleNote: '',
      reason: hint,
    };
  }

  const isUmlautDrop =
    ['j', 'q', 'x', 'y'].includes(cleanInitial) &&
    ['ü', 'üe', 'ün'].includes(cleanFinal);

  const writtenFinal = isUmlautDrop ? cleanFinal.replace('ü', 'u') : cleanFinal;
  const syllable = `${cleanInitial}${writtenFinal}`;
  const isWhole = WHOLE_SYLLABLES.has(syllable);

  let ruleNote = `标准拼读：${cleanInitial} + ${cleanFinal} → ${syllable}`;
  if (isUmlautDrop) {
    ruleNote = `触发「ü 去点魔法」：${cleanInitial} 遇到 ${cleanFinal} 去掉两点写成 ${syllable}，但读音仍是 ü！`;
  } else if (['n', 'l'].includes(cleanInitial) && cleanFinal.includes('ü')) {
    ruleNote = `触发「ü 保点规则」：${cleanInitial} 遇到 ${cleanFinal} 必须保留两点写成 ${syllable}！`;
  } else if (isWhole) {
    ruleNote = `「整体认读音节」：${syllable} 需要整体直呼记忆哦！`;
  }

  return {
    valid: true,
    syllable,
    initial: cleanInitial,
    medial: '',
    final: cleanFinal,
    isTriplet: false,
    isUmlautDrop,
    isWholeSyllable: isWhole,
    ruleNote,
  };
}

/**
 * Applies Mandarin tone mark following the official rule:
 * "有 a 别放过，没 a 找 o e，i u 并列标在后"
 */
export function applyToneMark(syllable: string, tone: ToneNumber): string {
  if (!syllable) return '';
  const idx = tone - 1;
  const chars = Array.from(syllable);

  const replaceAt = (pos: number) => {
    const ch = chars[pos];
    const mapped = TONE_VOWEL_MAP[ch]?.[idx];
    if (!mapped) return syllable;
    chars[pos] = mapped;
    return chars.join('');
  };

  // Priority 1: a
  const aPos = chars.indexOf('a');
  if (aPos !== -1) return replaceAt(aPos);

  // Priority 2: o or e
  const oPos = chars.indexOf('o');
  if (oPos !== -1) return replaceAt(oPos);
  const ePos = chars.indexOf('e');
  if (ePos !== -1) return replaceAt(ePos);

  // Priority 3: iu or ui -> mark the second vowel
  const lower = syllable.toLowerCase();
  if (lower.includes('iu')) {
    return replaceAt(chars.lastIndexOf('u'));
  }
  if (lower.includes('ui')) {
    return replaceAt(chars.lastIndexOf('i'));
  }

  // Priority 4: i, u, or ü
  for (const v of ['i', 'u', 'ü']) {
    const pos = chars.indexOf(v);
    if (pos !== -1) return replaceAt(pos);
  }

  return syllable;
}

export function getAvailableTones(syllable: string): ToneNumber[] {
  const tones = BAIDU_TONES[syllable];
  if (tones && tones.length > 0) {
    return tones.filter((t): t is ToneNumber => t >= 1 && t <= 4);
  }
  const curated = TONE_CHAR_MAP[syllable];
  if (curated) {
    const curatedTones = Object.keys(curated)
      .map(Number)
      .filter((t): t is ToneNumber => t >= 1 && t <= 4)
      .sort((a, b) => a - b);
    if (curatedTones.length > 0) return curatedTones;
  }
  return [1, 2, 3, 4];
}

export function getExampleChar(
  syllable: string,
  tone: ToneNumber
): { char: string; word: string } {
  const exact = TONE_CHAR_MAP[syllable]?.[tone];
  if (exact) return exact;
  const fallbackChar = SPEAK_MAP[syllable] ?? '音';
  return { char: fallbackChar, word: `${fallbackChar}字音` };
}

export function getSeedMnemonic(seedId: string, kind: 'initial' | 'final'): string {
  if (kind === 'initial') {
    return INITIAL_MNEMONICS[seedId] ?? `声母 ${seedId}`;
  }
  return FINAL_MNEMONICS[seedId] ?? `韵母 ${seedId}`;
}

/**
 * Returns prioritized real human audio URLs from flashtony2005/pinyin.
 * Only includes remote URLs known to exist in the repo's manifest (prevents 404s for triplets,
 * allowing audioPlayer to fall back smoothly to zh-CN SpeechSynthesis using the tone-matched char).
 */
export function getAudioUrls(key: string, tone?: ToneNumber): string[] {
  const encoded = encodeURIComponent(key);
  const urls: string[] = [];
  const repoTones = BAIDU_TONES[key];

  if (tone && repoTones && repoTones.includes(tone)) {
    urls.push(`${REPO_AUDIO_BASE}/bd/${encoded}${tone}.mp3`);
  }
  if (Object.prototype.hasOwnProperty.call(SPEAK_MAP, key)) {
    urls.push(`${REPO_AUDIO_BASE}/${encoded}.m4a`);
  }
  return urls;
}

/**
 * Deterministic visual hue for each brewed syllable potion bottle.
 */
export function getPotionColorHex(syllable: string, tone: ToneNumber): string {
  const toneColors: Record<ToneNumber, string[]> = {
    1: ['#2A7B88', '#3B8EA5', '#2F9B96'],
    2: ['#2E7D52', '#469663', '#5CA86F'],
    3: ['#C6861E', '#D49A2A', '#B57B24'],
    4: ['#B8473E', '#C85D44', '#A84255'],
  };
  let hash = 0;
  for (let i = 0; i < syllable.length; i++) {
    hash = (hash * 31 + syllable.charCodeAt(i)) % 97;
  }
  const palette = toneColors[tone];
  return palette[hash % palette.length];
}
