import pinyinTableRaw from './pinyin_table.json';
import baiduTonesRaw from './baidu_tones.json';
import speakMapRaw from './speak_map.json';

export type ToneNumber = 1 | 2 | 3 | 4;
export type SeasonId = 'spring' | 'summer' | 'autumn' | 'winter';
export type PotThemeId = 'terracotta' | 'celadon' | 'cobalt' | 'moss';
export type DecorId = 'auto_sprinkler' | 'firefly_lantern' | 'owl_echo_bell' | 'star_fertilizer';

export interface PinyinTableData {
  initials: string[];
  finals: string[];
  syllables: string[];
  matrix: Record<string, string[]>;
  zeroSyllables: string[];
}

export const PINYIN_TABLE: PinyinTableData = pinyinTableRaw as PinyinTableData;
export const BAIDU_TONES: Record<string, number[]> = baiduTonesRaw as Record<string, number[]>;
export const SPEAK_MAP: Record<string, string> = speakMapRaw as Record<string, string>;

export type MedialLetter = '' | 'i' | 'u' | 'ü';
export const MEDIAL_LETTERS: Exclude<MedialLetter, ''>[] = ['i', 'u', 'ü'];

/**
 * Standard Mandarin Three-Part Syllables (三拼音节: 声母 + 介母 i/u/ü + 韵母).
 * Combines the TRIPLETS table from flashtony2005/pinyin (src/pinyin.rs) plus standard
 * primary-school Mandarin i-medial / u-medial combinations (e.g. bian, tian, miao, zuo).
 */
export const TRIPLETS: ReadonlyArray<readonly [string, Exclude<MedialLetter, ''>, string]> = [
  // i 介母 — j / q / x
  ['j', 'i', 'a'], ['j', 'i', 'ao'], ['j', 'i', 'an'], ['j', 'i', 'ang'], ['j', 'i', 'ong'],
  ['q', 'i', 'a'], ['q', 'i', 'ao'], ['q', 'i', 'an'], ['q', 'i', 'ang'], ['q', 'i', 'ong'],
  ['x', 'i', 'a'], ['x', 'i', 'ao'], ['x', 'i', 'an'], ['x', 'i', 'ang'], ['x', 'i', 'ong'],
  // i 介母 — b / p / m / d / t / n / l (小学常见三拼音节)
  ['b', 'i', 'ao'], ['b', 'i', 'an'],
  ['p', 'i', 'ao'], ['p', 'i', 'an'],
  ['m', 'i', 'ao'], ['m', 'i', 'an'],
  ['d', 'i', 'ao'], ['d', 'i', 'an'],
  ['t', 'i', 'ao'], ['t', 'i', 'an'],
  ['n', 'i', 'ao'], ['n', 'i', 'an'], ['n', 'i', 'ang'],
  ['l', 'i', 'a'], ['l', 'i', 'ao'], ['l', 'i', 'an'], ['l', 'i', 'ang'],
  // ü 介母 — j / q / x (书写去两点: juan, quan, xuan)
  ['j', 'ü', 'an'], ['q', 'ü', 'an'], ['x', 'ü', 'an'],
  // u 介母 — g / k / h
  ['g', 'u', 'a'], ['g', 'u', 'o'], ['g', 'u', 'ai'], ['g', 'u', 'an'], ['g', 'u', 'ang'],
  ['k', 'u', 'a'], ['k', 'u', 'o'], ['k', 'u', 'ai'], ['k', 'u', 'an'], ['k', 'u', 'ang'],
  ['h', 'u', 'a'], ['h', 'u', 'o'], ['h', 'u', 'ai'], ['h', 'u', 'an'], ['h', 'u', 'ang'],
  // u 介母 — zh / ch / sh / r
  ['zh', 'u', 'a'], ['zh', 'u', 'o'], ['zh', 'u', 'ai'], ['zh', 'u', 'an'], ['zh', 'u', 'ang'],
  ['ch', 'u', 'a'], ['ch', 'u', 'o'], ['ch', 'u', 'ai'], ['ch', 'u', 'an'], ['ch', 'u', 'ang'],
  ['sh', 'u', 'a'], ['sh', 'u', 'o'], ['sh', 'u', 'ai'], ['sh', 'u', 'an'], ['sh', 'u', 'ang'],
  ['r', 'u', 'o'], ['r', 'u', 'an'],
  // u 介母 — d / t / n / l / z / c / s
  ['d', 'u', 'o'], ['d', 'u', 'an'],
  ['t', 'u', 'o'], ['t', 'u', 'an'],
  ['n', 'u', 'o'], ['n', 'u', 'an'],
  ['l', 'u', 'o'], ['l', 'u', 'an'],
  ['z', 'u', 'o'], ['z', 'u', 'an'],
  ['c', 'u', 'o'], ['c', 'u', 'an'],
  ['s', 'u', 'o'], ['s', 'u', 'an'],
];

/**
 * 23 Initial Mnemonics directly sourced from flashtony2005/pinyin (src/data/mnemonics.rs)
 */
export const INITIAL_MNEMONICS: Record<string, string> = {
  b: '收听广播 b b b，右下半圆 b b b',
  p: '右上半圆 p p p，端盆泼水 p p p',
  m: '两个门洞 m m m',
  f: '一根拐杖 f f f',
  d: '左下半圆 d d d，马蹄印儿 d d d',
  t: '伞柄朝下 t t t',
  n: '一个门洞 n n n',
  l: '一根小棍 l l l',
  g: '9 字加弯 g g g，白鸽飞翔 g g g',
  k: '蝌蚪戏水 k k k',
  h: '一把椅子 h h h',
  j: '竖弯加点 j j j，公鸡蝴蝶 j j j',
  q: '左上半圆 q q q，七个气球 q q q',
  x: '一个大叉 x x x，西瓜切片 x x x',
  zh: '蜘蛛结网 zh zh zh',
  ch: '小刺猬吃果 ch ch ch',
  sh: '狮子舞蹈 sh sh sh',
  r: '日出东方 r r r',
  z: '像个 2 字 z z z',
  c: '半个圆圈 c c c',
  s: '半个 8 字 s s s',
  y: '树杈 y y y',
  w: '两个屋顶 w w w',
};

/**
 * Child-friendly botanical mnemonics for all 24 Finals
 */
export const FINAL_MNEMONICS: Record<string, string> = {
  a: '张大嘴巴 a a a，晨露花瓣圆又大',
  o: '圆圆嘴巴 o o o，公鸡打鸣喔喔喔',
  e: '扁扁嘴巴 e e e，白鹅戏水清波里',
  i: '牙齿对齐 i i i，新芽顶着小水滴',
  u: '嘴巴突出 u u u，树洞猫头鹰呼呼',
  ü: '小鱼吐泡 ü ü ü，两点水珠亮晶晶',
  ai: '挨在一起 ai ai ai，紫藤花蔓并肩开',
  ei: '用力拔草 ei ei ei，小杯盛满青草露',
  ui: '围上围巾 ui ui ui，暖风吹过温室窗',
  ao: '身穿棉袄 ao ao ao，金色花苞暖洋洋',
  ou: '莲藕甜甜 ou ou ou，海鸥掠过玻璃顶',
  iu: '邮差送信 iu iu iu，柳枝轻拂小溪流',
  ie: '一片树叶 ie ie ie，月光洒在嫩叶上',
  üe: '弯弯月牙 üe üe üe，夜来香花静静开',
  er: '小小耳朵 er er er，卷舌轻声最特别',
  an: '天安门前 an an an，兰花幽香满花房',
  en: '轻轻摁铃 en en en，树根深深扎泥土',
  in: '森林绿荫 in in in，银铃叮当传佳音',
  un: '春风暖暖 un un un，嫩芽破土迎清晨',
  ün: '白云朵朵 ün ün ün，微风拂过苔藓裙',
  ang: '昂首向阳 ang ang ang，阳光洒满温室廊',
  eng: '蜜蜂采蜜 eng eng eng，台灯亮起夜未央',
  ing: '老鹰展翅 ing ing ing，萤火点亮玻璃瓶',
  ong: '红色灯笼 ong ong ong，松针清气绕花丛',
};

export interface PinyinRuleEntry {
  id: string;
  title: string;
  summary: string;
  examples: Array<{ label: string; syllable: string; tone?: ToneNumber }>;
}

/**
 * 5 Core Mandarin Pinyin Rules sourced from flashtony2005/pinyin (src/data/mnemonics.rs)
 */
export const PINYIN_RULES: PinyinRuleEntry[] = [
  {
    id: 'tones',
    title: '声调标调规则',
    summary:
      '普通话有四个声调：一声平（ā）、二声扬（á）、三声拐弯（ǎ）、四声降（à）。标调口诀：有 a 别放过，没 a 找 o e，i u 并列标在后。',
    examples: [
      { label: '妈 mā', syllable: 'ma', tone: 1 },
      { label: '麻 má', syllable: 'ma', tone: 2 },
      { label: '马 mǎ', syllable: 'ma', tone: 3 },
      { label: '骂 mà', syllable: 'ma', tone: 4 },
    ],
  },
  {
    id: 'umlaut',
    title: 'ü 的去点魔法规则',
    summary:
      '小鱼 ü 遇到 j、q、x、y 时会礼貌脱帽，去掉两点写成 u（ju、qu、xu、yu），读音仍然是 ü；遇到 n、l 时两点必须保留（nü、lü）。',
    examples: [
      { label: '居 jū', syllable: 'ju', tone: 1 },
      { label: '区 qū', syllable: 'qu', tone: 1 },
      { label: '需 xū', syllable: 'xu', tone: 1 },
      { label: '女 nǚ · 保点', syllable: 'nü', tone: 3 },
      { label: '绿 lǜ · 保点', syllable: 'lü', tone: 4 },
    ],
  },
  {
    id: 'whole',
    title: '整体认读音节',
    summary:
      'zhi、chi、shi、ri、zi、ci、si、yi、wu、yu、ye、yue、yuan、yin、yun、ying 这 16 个音节要整体记住，不用拆开生拼。',
    examples: [
      { label: '知 zhī', syllable: 'zhi', tone: 1 },
      { label: '吃 chī', syllable: 'chi', tone: 1 },
      { label: '狮 shī', syllable: 'shi', tone: 1 },
      { label: '鱼 yú', syllable: 'yu', tone: 2 },
      { label: '鹰 yīng', syllable: 'ying', tone: 1 },
    ],
  },
  {
    id: 'flat_retroflex',
    title: '平翘舌音辨析',
    summary:
      'z、c、s 是平舌音，舌尖平伸抵住上齿背；zh、ch、sh、r 是翘舌音，舌尖翘起抵住硬腭。秋季温室重点练习这两组声母！',
    examples: [
      { label: '字 zì', syllable: 'zi', tone: 4 },
      { label: '治 zhì', syllable: 'zhi', tone: 4 },
      { label: '四 sì', syllable: 'si', tone: 4 },
      { label: '十 shí', syllable: 'shi', tone: 2 },
    ],
  },
  {
    id: 'nasal',
    title: '前鼻音与后鼻音',
    summary:
      'an、en、in、un、ün 是前鼻韵母，收尾舌尖顶上齿龈；ang、eng、ing、ong 是后鼻韵母，舌根后缩共鸣更浑厚。',
    examples: [
      { label: '兰 lán', syllable: 'lan', tone: 2 },
      { label: '狼 láng', syllable: 'lang', tone: 2 },
      { label: '林 lín', syllable: 'lin', tone: 2 },
      { label: '铃 líng', syllable: 'ling', tone: 2 },
    ],
  },
];

export const WHOLE_SYLLABLES = new Set([
  'zhi', 'chi', 'shi', 'ri', 'zi', 'ci', 'si',
  'yi', 'wu', 'yu', 'ye', 'yue', 'yuan', 'yin', 'yun', 'ying',
]);

/**
 * Curated tone-aware example characters for rich learning clues in orders & potion labels.
 */
export const TONE_CHAR_MAP: Record<string, Partial<Record<ToneNumber, { char: string; word: string }>>> = {
  ba: {
    1: { char: '八', word: '八片叶' },
    2: { char: '拔', word: '拔萝卜' },
    3: { char: '把', word: '火把' },
    4: { char: '爸', word: '爸爸' },
  },
  bo: {
    1: { char: '波', word: '清波' },
    2: { char: '伯', word: '树伯' },
    3: { char: '跛', word: '慢步' },
    4: { char: '播', word: '播种' },
  },
  ma: {
    1: { char: '妈', word: '妈妈' },
    2: { char: '麻', word: '芝麻' },
    3: { char: '马', word: '白马' },
    4: { char: '骂', word: '喧闹' },
  },
  mi: {
    1: { char: '咪', word: '猫咪' },
    2: { char: '迷', word: '迷雾' },
    3: { char: '米', word: '稻米' },
    4: { char: '蜜', word: '花蜜' },
  },
  mu: {
    2: { char: '模', word: '模样' },
    3: { char: '母', word: '贝母' },
    4: { char: '木', word: '古木' },
  },
  fa: {
    1: { char: '发', word: '发芽' },
    2: { char: '乏', word: '解乏' },
    3: { char: '法', word: '魔法' },
    4: { char: '发', word: '青丝' },
  },
  fu: {
    1: { char: '敷', word: '敷药' },
    2: { char: '福', word: '福气' },
    3: { char: '府', word: '花府' },
    4: { char: '富', word: '丰盛' },
  },
  da: {
    1: { char: '搭', word: '搭棚' },
    2: { char: '答', word: '回答' },
    3: { char: '打', word: '打水' },
    4: { char: '大', word: '大树' },
  },
  di: {
    1: { char: '滴', word: '露滴' },
    2: { char: '笛', word: '竹笛' },
    3: { char: '底', word: '谷底' },
    4: { char: '地', word: '大地' },
  },
  tu: {
    1: { char: '突', word: '破土' },
    2: { char: '图', word: '图鉴' },
    3: { char: '土', word: '沃土' },
    4: { char: '兔', word: '玉兔' },
  },
  ni: {
    1: { char: '妮', word: '小妮' },
    2: { char: '泥', word: '花泥' },
    3: { char: '你', word: '你好' },
    4: { char: '腻', word: '细腻' },
  },
  nü: {
    3: { char: '女', word: '花仙女' },
  },
  lü: {
    2: { char: '驴', word: '小毛驴' },
    3: { char: '旅', word: '旅行' },
    4: { char: '绿', word: '翠绿' },
  },
  lu: {
    1: { char: '噜', word: '呼噜' },
    2: { char: '炉', word: '药炉' },
    3: { char: '鲁', word: '朴实' },
    4: { char: '露', word: '晨露' },
  },
  li: {
    1: { char: '哩', word: '清脆' },
    2: { char: '梨', word: '雪梨' },
    3: { char: '礼', word: '礼物' },
    4: { char: '力', word: '活力' },
  },
  pa: {
    1: { char: '趴', word: '趴着' },
    2: { char: '爬', word: '爬藤' },
    4: { char: '怕', word: '不怕' },
  },
  po: {
    1: { char: '坡', word: '山坡' },
    2: { char: '婆', word: '外婆' },
    3: { char: '叵', word: '奇思' },
    4: { char: '破', word: '破晓' },
  },
  ge: {
    1: { char: '歌', word: '歌声' },
    2: { char: '格', word: '花格' },
    3: { char: '葛', word: '葛藤' },
    4: { char: '个', word: '个头' },
  },
  gu: {
    1: { char: '菇', word: '蘑菇' },
    2: { char: '古', word: '古藤' },
    3: { char: '谷', word: '山谷' },
    4: { char: '故', word: '故事' },
  },
  hu: {
    1: { char: '呼', word: '呼吸' },
    2: { char: '狐', word: '红狐' },
    3: { char: '虎', word: '小虎' },
    4: { char: '护', word: '守护' },
  },
  he: {
    1: { char: '喝', word: '喝茶' },
    2: { char: '河', word: '小河' },
    4: { char: '贺', word: '祝贺' },
  },
  ju: {
    1: { char: '居', word: '隐居' },
    2: { char: '菊', word: '秋菊' },
    3: { char: '举', word: '举灯' },
    4: { char: '聚', word: '相聚' },
  },
  qu: {
    1: { char: '驱', word: '驱雾' },
    2: { char: '渠', word: '水渠' },
    3: { char: '曲', word: '小曲' },
    4: { char: '趣', word: '趣味' },
  },
  xu: {
    1: { char: '须', word: '根须' },
    2: { char: '徐', word: '微风徐' },
    3: { char: '许', word: '许愿' },
    4: { char: '序', word: '四季序' },
  },
  kai: {
    1: { char: '开', word: '花开' },
    3: { char: '凯', word: '凯旋' },
  },
  hao: {
    1: { char: '蒿', word: '青蒿' },
    2: { char: '毫', word: '画毫' },
    3: { char: '好', word: '好运' },
    4: { char: '浩', word: '浩瀚' },
  },
  bai: {
    1: { char: '掰', word: '采摘' },
    2: { char: '白', word: '白云' },
    3: { char: '百', word: '百花' },
    4: { char: '拜', word: '拜访' },
  },
  mei: {
    2: { char: '梅', word: '寒梅' },
    3: { char: '美', word: '美丽' },
    4: { char: '魅', word: '魔力' },
  },
  lan: {
    2: { char: '兰', word: '幽兰' },
    3: { char: '懒', word: '慵懒' },
    4: { char: '烂', word: '灿烂' },
  },
  feng: {
    1: { char: '风', word: '清风' },
    2: { char: '逢', word: '相逢' },
    3: { char: '讽', word: '吟诵' },
    4: { char: '凤', word: '凤仙' },
  },
  qing: {
    1: { char: '青', word: '青苔' },
    2: { char: '晴', word: '晴空' },
    3: { char: '请', word: '邀请' },
    4: { char: '庆', word: '庆典' },
  },
  shu: {
    1: { char: '书', word: '药书' },
    2: { char: '熟', word: '成熟' },
    3: { char: '鼠', word: '松鼠' },
    4: { char: '树', word: '大树' },
  },
  zhi: {
    1: { char: '枝', word: '花枝' },
    2: { char: '植', word: '植物' },
    3: { char: '纸', word: '羊皮纸' },
    4: { char: '治', word: '治愈' },
  },
  cha: {
    1: { char: '插', word: '插花' },
    2: { char: '茶', word: '花茶' },
  },
  shui: {
    2: { char: '谁', word: '来客' },
    3: { char: '水', word: '泉水' },
    4: { char: '睡', word: '安睡' },
  },
  yu: {
    1: { char: '淤', word: '泥塘' },
    2: { char: '鱼', word: '游鱼' },
    3: { char: '雨', word: '春雨' },
    4: { char: '玉', word: '碧玉' },
  },
  wu: {
    1: { char: '屋', word: '花屋' },
    2: { char: '梧', word: '梧桐' },
    3: { char: '舞', word: '蝶舞' },
    4: { char: '雾', word: '晨雾' },
  },
  a: {
    1: { char: '阿', word: '阿婆' },
    2: { char: '啊', word: '惊奇' },
    3: { char: '啊', word: '疑问' },
    4: { char: '啊', word: '欢呼' },
  },
  ai: {
    1: { char: '哀', word: '轻叹' },
    2: { char: '皑', word: '白雪' },
    3: { char: '矮', word: '矮灌木' },
    4: { char: '爱', word: '喜爱' },
  },
  er: {
    2: { char: '儿', word: '花儿' },
    3: { char: '耳', word: '木耳' },
    4: { char: '二', word: '二叶草' },
  },
  // Three-part syllables (三拼音节常用例字)
  jia: {
    1: { char: '家', word: '温室家' },
    2: { char: '夹', word: '花夹' },
    3: { char: '甲', word: '甲虫' },
    4: { char: '架', word: '花架' },
  },
  qia: {
    1: { char: '掐', word: '掐芽' },
    3: { char: '卡', word: '发卡' },
    4: { char: '恰', word: '恰好' },
  },
  xia: {
    1: { char: '虾', word: '小虾' },
    2: { char: '霞', word: '晚霞' },
    4: { char: '夏', word: '夏日' },
  },
  jiao: {
    1: { char: '浇', word: '浇花' },
    2: { char: '嚼', word: '细嚼' },
    3: { char: '角', word: '花角' },
    4: { char: '叫', word: '鸟叫' },
  },
  qiao: {
    1: { char: '敲', word: '敲门' },
    2: { char: '桥', word: '木桥' },
    3: { char: '巧', word: '灵巧' },
    4: { char: '俏', word: '俏丽' },
  },
  xiao: {
    1: { char: '消', word: '消雾' },
    3: { char: '小', word: '小芽' },
    4: { char: '笑', word: '微笑' },
  },
  jian: {
    1: { char: '尖', word: '叶尖' },
    3: { char: '剪', word: '花剪' },
    4: { char: '见', word: '相见' },
  },
  qian: {
    1: { char: '千', word: '千朵' },
    2: { char: '前', word: '窗前' },
    3: { char: '浅', word: '浅绿' },
    4: { char: '欠', word: '呵欠' },
  },
  xian: {
    1: { char: '仙', word: '水仙' },
    2: { char: '闲', word: '悠闲' },
    3: { char: '显', word: '显现' },
    4: { char: '线', word: '丝线' },
  },
  jiang: {
    1: { char: '江', word: '清江' },
    3: { char: '讲', word: '讲故事' },
    4: { char: '降', word: '霜降' },
  },
  qiang: {
    1: { char: '蔷', word: '蔷薇' },
    2: { char: '墙', word: '石墙' },
    3: { char: '抢', word: '抢收' },
    4: { char: '呛', word: '花香' },
  },
  xiang: {
    1: { char: '香', word: '花香' },
    2: { char: '祥', word: '吉祥' },
    3: { char: '想', word: '梦想' },
    4: { char: '向', word: '向阳' },
  },
  jiong: {
    3: { char: '炯', word: '炯炯' },
  },
  qiong: {
    2: { char: '琼', word: '琼浆' },
  },
  xiong: {
    1: { char: '兄', word: '师兄' },
    2: { char: '熊', word: '小熊' },
  },
  juan: {
    1: { char: '捐', word: '赠予' },
    3: { char: '卷', word: '卷叶' },
    4: { char: '倦', word: '微倦' },
  },
  quan: {
    1: { char: '圈', word: '花圈' },
    2: { char: '泉', word: '清泉' },
    3: { char: '犬', word: '小犬' },
    4: { char: '劝', word: '叮嘱' },
  },
  xuan: {
    1: { char: '轩', word: '花轩' },
    2: { char: '玄', word: '玄妙' },
    3: { char: '选', word: '挑选' },
    4: { char: '炫', word: '绚丽' },
  },
  gua: {
    1: { char: '瓜', word: '甜瓜' },
    3: { char: '寡', word: '清淡' },
    4: { char: '挂', word: '挂灯' },
  },
  kua: {
    1: { char: '夸', word: '夸奖' },
    3: { char: '垮', word: '松土' },
    4: { char: '跨', word: '跨越' },
  },
  hua: {
    1: { char: '花', word: '灵花' },
    2: { char: '华', word: '芳华' },
    4: { char: '画', word: '水彩画' },
  },
  guo: {
    1: { char: '锅', word: '药锅' },
    2: { char: '国', word: '花国' },
    3: { char: '果', word: '果实' },
    4: { char: '过', word: '路过' },
  },
  huo: {
    1: { char: '豁', word: '开朗' },
    2: { char: '活', word: '活力' },
    3: { char: '火', word: '炉火' },
    4: { char: '获', word: '收获' },
  },
  duo: {
    1: { char: '多', word: '丰收' },
    2: { char: '夺', word: '夺目' },
    3: { char: '朵', word: '花朵' },
    4: { char: '舵', word: '风舵' },
  },
  tuo: {
    1: { char: '托', word: '花托' },
    2: { char: '驼', word: '驼铃' },
    3: { char: '妥', word: '妥帖' },
    4: { char: '拓', word: '拓印' },
  },
  nuo: {
    2: { char: '挪', word: '挪盆' },
    4: { char: '诺', word: '承诺' },
  },
  luo: {
    1: { char: '啰', word: '轻吟' },
    2: { char: '萝', word: '绿萝' },
    3: { char: '裸', word: '裸根' },
    4: { char: '落', word: '花落' },
  },
  lia: {
    3: { char: '俩', word: '哥俩' },
  },
  tian: {
    1: { char: '天', word: '晴天' },
    2: { char: '甜', word: '香甜' },
    3: { char: '舔', word: '轻尝' },
  },
  niao: {
    3: { char: '鸟', word: '林鸟' },
  },
  miao: {
    1: { char: '喵', word: '小猫' },
    2: { char: '苗', word: '幼苗' },
    3: { char: '秒', word: '分秒' },
    4: { char: '妙', word: '奇妙' },
  },
  bian: {
    1: { char: '边', word: '窗边' },
    3: { char: '扁', word: '扁豆' },
    4: { char: '变', word: '变化' },
  },
  zhuang: {
    1: { char: '装', word: '装扮' },
    3: { char: '奘', word: '厚实' },
    4: { char: '壮', word: '茁壮' },
  },
  huang: {
    1: { char: '荒', word: '田野' },
    2: { char: '黄', word: '鹅黄' },
    3: { char: '晃', word: '摇晃' },
  },
  shuo: {
    1: { char: '说', word: '细说' },
    4: { char: '硕', word: '硕果' },
  },
  zuo: {
    1: { char: '作', word: '花坊' },
    2: { char: '昨', word: '昨日' },
    3: { char: '左', word: '左侧' },
    4: { char: '坐', word: '小坐' },
  },
};

export interface SeasonDefinition {
  id: SeasonId;
  name: string;
  subtitle: string;
  requiredOrders: number;
  initials: string[];
  finals: string[];
  accentHex: string;
}

export const SEASONS: SeasonDefinition[] = [
  {
    id: 'spring',
    name: '春季 · 单韵母与三拼初芽',
    subtitle: '掌握 b p m f d t n l j q x 与单韵母，体验两拼及 j-i-a 等基础三拼',
    requiredOrders: 0,
    initials: ['b', 'p', 'm', 'f', 'd', 't', 'n', 'l', 'j', 'q', 'x'],
    finals: ['a', 'o', 'e', 'i', 'u', 'ü'],
    accentHex: '#2E6F52',
  },
  {
    id: 'summer',
    name: '夏季 · 复韵母盛放',
    subtitle: '解锁 g k h 与复韵母，炼制 guā、huā、xiǎo 等夏日三拼魔药',
    requiredOrders: 3,
    initials: ['g', 'k', 'h'],
    finals: ['ai', 'ei', 'ui', 'ao', 'ou', 'iu', 'ie', 'üe', 'er'],
    accentHex: '#C6861E',
  },
  {
    id: 'autumn',
    name: '秋季 · 鼻韵母丰收',
    subtitle: '挑战平翘舌 zh ch sh r z c s 与前后鼻韵母',
    requiredOrders: 7,
    initials: ['zh', 'ch', 'sh', 'r', 'z', 'c', 's'],
    finals: ['an', 'en', 'in', 'un', 'ün', 'ang', 'eng', 'ing', 'ong'],
    accentHex: '#B85A34',
  },
  {
    id: 'winter',
    name: '冬季 · 全音节大师',
    subtitle: '解锁 y w 零声母与全部 343 个普通话标准音节',
    requiredOrders: 12,
    initials: ['y', 'w'],
    finals: [],
    accentHex: '#3B6E8C',
  },
];

export interface DecorItem {
  id: DecorId;
  name: string;
  effect: string;
  cost: number;
}

export const DECOR_CATALOG: DecorItem[] = [
  {
    id: 'auto_sprinkler',
    name: '铜制自动洒水器',
    effect: '每日清晨自动为温室所有幼苗浇水 1 次',
    cost: 45,
  },
  {
    id: 'firefly_lantern',
    name: '萤火虫玻璃提灯',
    effect: '每日最大精力上限 +4（6 → 10）',
    cost: 55,
  },
  {
    id: 'owl_echo_bell',
    name: '猫头鹰回音风铃',
    effect: '听音委托可免费展开声母/韵母拼读提示',
    cost: 40,
  },
  {
    id: 'star_fertilizer',
    name: '星尘营养花土',
    effect: '每次收获额外获得 +2 份声母/韵母素材',
    cost: 65,
  },
];

export interface PotTheme {
  id: PotThemeId;
  name: string;
  bodyHex: string;
  rimHex: string;
  shadowHex: string;
  cost: number;
}

export const POT_THEMES: PotTheme[] = [
  {
    id: 'terracotta',
    name: '经典红陶盆',
    bodyHex: '#C96F4B',
    rimHex: '#DF8561',
    shadowHex: '#9C4D2E',
    cost: 0,
  },
  {
    id: 'celadon',
    name: '宋韵青瓷盆',
    bodyHex: '#7FA594',
    rimHex: '#9EC2B1',
    shadowHex: '#567868',
    cost: 30,
  },
  {
    id: 'cobalt',
    name: '星夜青花盆',
    bodyHex: '#46688B',
    rimHex: '#6389B0',
    shadowHex: '#2C4663',
    cost: 35,
  },
  {
    id: 'moss',
    name: '森林苔石盆',
    bodyHex: '#6E7559',
    rimHex: '#8C9474',
    shadowHex: '#4B523B',
    cost: 30,
  },
];

export interface VillagerProfile {
  id: string;
  name: string;
  role: string;
  avatar: string;
}

export const VILLAGERS: VillagerProfile[] = [
  {
    id: 'owl',
    name: '巴布邮差',
    role: '森林信使',
    avatar: '/src/assets/images/villager_owl_1795426524588.jpg',
  },
  {
    id: 'fox',
    name: '洛洛画师',
    role: '植物绘师',
    avatar: '/src/assets/images/villager_fox_1795426525501.jpg',
  },
  {
    id: 'frog',
    name: '咕咕乐手',
    role: '荷塘歌者',
    avatar: '/src/assets/images/villager_frog_1795426524076.jpg',
  },
];

export const GREENHOUSE_IMAGES = {
  titleBg: '/src/assets/images/title_greenhouse_bg_1795426524992.jpg',
  backdrop: '/src/assets/images/greenhouse_backdrop_1795426525222.jpg',
  witch: '/src/assets/images/witch_girl_cut_1795426526001.png',
};
