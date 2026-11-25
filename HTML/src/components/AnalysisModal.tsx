import React from 'react';
import { X } from 'lucide-react';

interface AnalysisModalProps {
  onClose: () => void;
}

const MAPPING_ROWS = [
  {
    mg: '24 种魔法植物种植、浇水精力管理与收获',
    pinyin: '23 声母晶芽 + 24 韵母灵花；浇水即播放真人读音并展示 mnemonics.rs 顺口溜',
    asset: 'pinyin_table.json + mnemonics.rs + assets/audio/*.m4a',
  },
  {
    mg: '炼金蒸馏台：按配方酿造 + 探索式试错发现 70 种魔药',
    pinyin: '投入声母+韵母+四声火候；合法组合酿出 343 个标准音节，非法组合(如 b+ong)冒烟提示规则',
    asset: 'pinyin_table.json (matrix) + src/pinyin.rs (ü 去点规则)',
  },
  {
    mg: '85+ 森林村民奇特委托，交付对应药水推进剧情',
    pinyin: '听音配药 / 看字拼读 / 规则挑战三类委托，支持慢速朗读与回音风铃提示',
    asset: 'assets/audio/bd/{syllable}{tone}.mp3 (1069 真人声调音频) + speak_map.json',
  },
  {
    mg: '四季推进解锁新植物 + 15 种功能性温室装饰',
    pinyin: '春(单韵母) → 夏(复韵母/jqx+ü) → 秋(平翘舌/鼻韵母) → 冬(全音节)；自动洒水器/提灯/釉色花盆',
    asset: 'baidu_tones.json + 课程分阶解锁体系',
  },
];

export const AnalysisModal: React.FC<AnalysisModalProps> = ({ onClose }) => {
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Magical Greenhouse 与 flashtony2005/pinyin 资源结合可行性分析"
      className="fixed inset-0 z-50 flex items-center justify-center bg-[#141D15]/75 p-4"
    >
      <div className="flex max-h-[86vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-[#C7BEA8] bg-[#FAF7EE] shadow-2xl">
        <div className="flex items-center justify-between border-b border-[#DFD8C6] bg-[#243126] px-6 py-4 text-[#FAF5E8]">
          <div>
            <h2 className="font-display text-xl font-bold">
              《Magical Greenhouse》× pinyin 项目融合可行性分析
            </h2>
            <p className="text-xs text-[#D5CFC0]">
              结论：结构高度同构，可直接复用该仓库的 1435 个离线音频与标准普通话音节矩阵
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭分析面板"
            className="flex h-9 w-9 items-center justify-center rounded-lg border border-[#4D604F] bg-[#304032] text-[#EFE8D8] hover:bg-[#3B4F3E]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 space-y-5 overflow-y-auto p-6 text-sm text-[#2A332B]">
          <div className="rounded-xl border border-[#C7D7CB] bg-[#EFF6F1] p-4">
            <h3 className="font-bold text-[#1E5239]">为什么这两个项目天然契合？</h3>
            <p className="mt-1.5 text-xs leading-relaxed text-[#35463A]">
              Steam 游戏《Magical Greenhouse》(AppID 3837530) 的核心乐趣不是高压力时间管理，而是
              <strong>「低压力培育植物 → 探索式炼金试错 → 收集图鉴 → 满足村民订单 → 装扮温室」</strong>
              的治愈循环。这恰恰解决了儿童拼音练习最大的痛点——枯燥重复：把「声母/韵母」变成要照料的植物，把「拼读规则与合法性校验」变成炼金实验的探索惊喜，再把「听音辨调」包装成可爱的森林村民订单！
            </p>
          </div>

          <div>
            <h3 className="mb-2.5 font-bold text-[#222B23]">核心玩法 × 开源资源 1:1 映射表</h3>
            <div className="overflow-x-auto rounded-xl border border-[#DDD5C3] bg-[#FFFDF8]">
              <table className="w-full border-collapse text-left text-xs">
                <thead>
                  <tr className="border-b border-[#E3DCCB] bg-[#F3EFE3] text-[#4A534B]">
                    <th className="px-3.5 py-2.5 font-semibold">Magical Greenhouse 机制</th>
                    <th className="px-3.5 py-2.5 font-semibold">本原型拼音融合设计</th>
                    <th className="px-3.5 py-2.5 font-semibold">复用的 flashtony2005/pinyin 资源</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#EBE4D4]">
                  {MAPPING_ROWS.map((row) => (
                    <tr key={row.mg}>
                      <td className="px-3.5 py-3 align-top font-medium text-[#2A332B]">{row.mg}</td>
                      <td className="px-3.5 py-3 align-top text-[#3B463C]">{row.pinyin}</td>
                      <td className="font-mono-num px-3.5 py-3 align-top text-[11px] text-[#1F583D]">
                        {row.asset}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-[#DDD5C3] bg-[#FFFDF8] p-4">
              <h4 className="text-xs font-bold text-[#222B23]">本原型已落地的资源复用</h4>
              <ul className="mt-2 list-disc space-y-1 pl-4 text-xs text-[#4C554C]">
                <li>已完整导入 `pinyin_table.json`（23 声母、24 韵母、343 合法音节矩阵）</li>
                <li>已导入 `baidu_tones.json` 与 `speak_map.json` 校验真实声调与汉字例字</li>
                <li>已直接挂载仓库内 1,069 个真人百度汉语声调 MP3 + 367 个 M4A 音频</li>
                <li>已移植 `mnemonics.rs` 23 条声母顺口溜与 5 大拼音规则</li>
              </ul>
            </div>

            <div className="rounded-xl border border-[#DDD5C3] bg-[#FFFDF8] p-4">
              <h4 className="text-xs font-bold text-[#222B23]">下一步进阶扩展方向</h4>
              <ul className="mt-2 list-disc space-y-1 pl-4 text-xs text-[#4C554C]">
                <li>
                  <strong>麦克风跟读催熟</strong>：复用原仓库 `audio_tone.rs` + `dtw.rs` 音高识别算法，孩子对着麦克风读准声调就能免费给植物施魔法加速！
                </li>
                <li>
                  <strong>三拼音节高阶炉</strong>：引入 `src/pinyin.rs` 中的 `TRIPLETS` 三拼音节表（声母+介母 i/u/ü+韵母），作为高阶「三管蒸馏器」玩法。
                </li>
                <li>
                  <strong>离线打包</strong>：原仓库全部 1,435 个音频仅 4.2 MB，非常适合做离线 PWA 或平板儿童应用。
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
