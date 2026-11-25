import React, { useState } from 'react';
import { BookOpen, Sparkles, Volume2, X } from 'lucide-react';
import {
  PINYIN_RULES,
  type ToneNumber,
} from '../data/pinyinResources';
import {
  TOTAL_DISCOVERABLE_SYLLABLES,
  applyToneMark,
  getExampleChar,
} from '../game/pinyinEngine';

interface GrimoireModalProps {
  discoveredSyllables: string[];
  discoveredTones: string[];
  onPlayAudio: (syllable: string, tone?: ToneNumber) => void;
  onClose: () => void;
}

export const GrimoireModal: React.FC<GrimoireModalProps> = ({
  discoveredSyllables,
  discoveredTones,
  onPlayAudio,
  onClose,
}) => {
  const [tab, setTab] = useState<'discovered' | 'rules'>('discovered');

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="拼音炼金药典"
      className="fixed inset-0 z-40 flex items-center justify-center bg-[#141D15]/70 p-4"
    >
      <div className="flex max-h-[86vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-[#C7BEA8] bg-[#FAF7EE] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#DFD8C6] bg-[#243126] px-6 py-4 text-[#FAF5E8]">
          <div>
            <h2 className="font-display text-xl font-bold">拼音炼金药典</h2>
            <p className="text-xs text-[#D5CFC0]">
              已收录 {discoveredSyllables.length} / {TOTAL_DISCOVERABLE_SYLLABLES} 个普通话标准音节（含三拼音节）
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭药典"
            className="flex h-9 w-9 items-center justify-center rounded-lg border border-[#4D604F] bg-[#304032] text-[#EFE8D8] hover:bg-[#3B4F3E]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex items-center gap-2 border-b border-[#E3DCCB] bg-[#F2EDE0] px-6 py-2.5">
          <button
            type="button"
            onClick={() => setTab('discovered')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold transition-colors whitespace-nowrap ${
              tab === 'discovered'
                ? 'bg-[#2E6F52] text-[#FFFDF8]'
                : 'text-[#4E564E] hover:bg-[#E5DEC9]'
            }`}
          >
            <Sparkles className="h-3.5 w-3.5" />
            <span>已发现魔药 ({discoveredTones.length})</span>
          </button>
          <button
            type="button"
            onClick={() => setTab('rules')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold transition-colors whitespace-nowrap ${
              tab === 'rules'
                ? 'bg-[#2E6F52] text-[#FFFDF8]'
                : 'text-[#4E564E] hover:bg-[#E5DEC9]'
            }`}
          >
            <BookOpen className="h-3.5 w-3.5" />
            <span>五大拼音魔法规则</span>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-6">
          {tab === 'discovered' ? (
            discoveredTones.length === 0 ? (
              <div className="rounded-xl border border-[#DDD5C3] bg-[#FFFDF8] p-8 text-center">
                <p className="text-sm font-medium text-[#3C463C]">药典目前仍是空白书页</p>
                <p className="mt-1 text-xs text-[#6B7166]">
                  前往炼金蒸馏台组合任意合法的声母、韵母与声调，炼出的新音节都会永久记录在这里并可随时点击发声！
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                {discoveredTones.map((key) => {
                  const [syllable, toneRaw] = key.split(':');
                  const tone = Number(toneRaw) as ToneNumber;
                  const marked = applyToneMark(syllable, tone);
                  const { char, word } = getExampleChar(syllable, tone);
                  return (
                    <button
                      key={key}
                      type="button"
                      onClick={() => onPlayAudio(syllable, tone)}
                      className="flex items-center justify-between rounded-xl border border-[#D8D0BE] bg-[#FFFDF8] px-3.5 py-2.5 text-left transition-colors hover:border-[#2E6F52] hover:bg-[#F0F7F2]"
                    >
                      <div>
                        <div className="font-display text-lg font-bold text-[#1F583D]">
                          {marked}
                        </div>
                        <div className="text-xs text-[#5E655C]">
                          {char} · {word}
                        </div>
                      </div>
                      <Volume2 className="h-4 w-4 shrink-0 text-[#5C7968]" />
                    </button>
                  );
                })}
              </div>
            )
          ) : (
            <div className="space-y-4">
              {PINYIN_RULES.map((rule, idx) => (
                <article
                  key={rule.id}
                  className="rounded-xl border border-[#DDD5C3] bg-[#FFFDF8] p-4"
                >
                  <h3 className="text-sm font-bold text-[#222B23]">
                    0{idx + 1}. {rule.title}
                  </h3>
                  <p className="mt-1.5 text-xs leading-relaxed text-[#4C554C]">{rule.summary}</p>
                  <div className="mt-3 flex flex-wrap gap-2">
                    {rule.examples.map((ex) => (
                      <button
                        key={ex.label}
                        type="button"
                        onClick={() => onPlayAudio(ex.syllable, ex.tone)}
                        className="inline-flex items-center gap-1.5 rounded-lg border border-[#CBD7CE] bg-[#F2F7F3] px-3 py-1.5 text-xs font-medium text-[#1F583D] hover:bg-[#E1EFE5]"
                      >
                        <Volume2 className="h-3.5 w-3.5" />
                        <span>{ex.label}</span>
                      </button>
                    ))}
                  </div>
                </article>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
