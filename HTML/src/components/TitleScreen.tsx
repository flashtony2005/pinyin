import React, { useEffect } from 'react';
import { BookOpen, Play, Volume2, VolumeX } from 'lucide-react';
import { GREENHOUSE_IMAGES } from '../data/pinyinResources';

interface TitleScreenProps {
  onStart: () => void;
  onOpenAnalysis: () => void;
  muted: boolean;
  onToggleMute: () => void;
}

export const TitleScreen: React.FC<TitleScreenProps> = ({
  onStart,
  onOpenAnalysis,
  muted,
  onToggleMute,
}) => {
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onStart();
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onStart]);

  return (
    <div className="relative min-h-screen w-full overflow-hidden bg-[#182319] text-[#F7F3E8] select-none">
      {/* Full-bleed Illustrated Key Art Backdrop */}
      <img
        src={GREENHOUSE_IMAGES.titleBg}
        alt="阳光洒落的魔法拼音温室与炼金蒸馏台"
        referrerPolicy="no-referrer"
        className="absolute inset-0 h-full w-full object-cover object-center"
      />

      {/* Measured contrast scrim anchored on the calm left negative space */}
      <div
        className="absolute inset-0"
        style={{
          background:
            'linear-gradient(100deg, rgba(16, 26, 18, 0.90) 0%, rgba(19, 31, 21, 0.72) 40%, rgba(18, 28, 20, 0.22) 72%, rgba(18, 28, 20, 0.35) 100%)',
        }}
      />

      {/* Character key art: the little witch as a cut-out, standing at the
          bottom-right of the scene (transparent PNG, no frame) */}
      <div className="pointer-events-none absolute bottom-0 right-2 z-[5] sm:right-6 lg:right-12">
        {/* warm lantern glow behind her */}
        <div
          className="absolute bottom-4 left-1/2 -z-10 h-[62%] w-[85%] -translate-x-1/2 blur-3xl"
          style={{
            background:
              'radial-gradient(ellipse 55% 55% at 50% 60%, rgba(255, 206, 128, 0.42) 0%, rgba(255, 190, 110, 0.16) 48%, rgba(255, 190, 110, 0) 76%)',
          }}
        />
        <img
          src={GREENHOUSE_IMAGES.witch}
          alt="手持魔杖的小女巫，站在洒满星光的温室里"
          referrerPolicy="no-referrer"
          className="block h-auto w-[9.5rem] select-none drop-shadow-[0_16px_28px_rgba(0,0,0,0.55)] sm:w-[12.5rem] lg:w-[17rem]"
        />
      </div>

      {/* Top-right minimal sound toggle */}
      <div className="relative z-20 flex justify-end p-6">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            onToggleMute();
          }}
          aria-label={muted ? '开启声音' : '静音'}
          className="flex h-11 w-11 items-center justify-center rounded-xl border border-[#E5D7B8]/30 bg-[#1B271D]/85 text-[#F4EAD5] shadow-md transition-transform duration-150 hover:scale-105 hover:border-[#E5D7B8]/60 focus-visible:outline-2 focus-visible:outline-[#E0B15E]"
        >
          {muted ? <VolumeX className="h-5 w-5" /> : <Volume2 className="h-5 w-5" />}
        </button>
      </div>

      {/* Left-third negative-space title emblem & primary CTA */}
      <main className="relative z-10 flex min-h-[calc(100vh-116px)] max-w-6xl flex-col justify-center px-8 pb-12 sm:px-14 lg:px-20">
        <div className="max-w-lg">
          {/* Bespoke inline SVG Botanical Crest & Title Lockup */}
          <svg
            viewBox="0 0 420 132"
            className="mb-6 w-full max-w-[420px] drop-shadow-[0_6px_18px_rgba(0,0,0,0.55)]"
            role="img"
            aria-label="魔法拼音温室 Pinyin Greenhouse"
          >
            {/* Ornamental botanical arch frame */}
            <path
              d="M 18 32 Q 18 12 40 12 L 154 12 M 266 12 L 380 12 Q 402 12 402 32"
              fill="none"
              stroke="#D8B36A"
              strokeWidth="1.75"
              strokeLinecap="round"
              opacity="0.85"
            />
            {/* Center sprouting leaf & alembic emblem */}
            <g transform="translate(210, 14)">
              <circle cx="0" cy="0" r="10" fill="#233726" stroke="#D8B36A" strokeWidth="1.6" />
              <path
                d="M 0 5 L 0 -5 M 0 0 C -5 -3 -6 -7 -1 -8 C 0 -4 0 -2 0 0 Z M 0 -1 C 5 -4 6 -7 1 -8 C 0 -5 0 -3 0 -1 Z"
                fill="#8BC37A"
                stroke="#E9D6A6"
                strokeWidth="1"
              />
            </g>

            {/* Primary Chinese Title */}
            <text
              x="210"
              y="74"
              textAnchor="middle"
              fill="#FAF5E6"
              stroke="#19241B"
              strokeWidth="5"
              paintOrder="stroke fill"
              style={{
                fontFamily: "'Noto Sans SC', serif",
                fontWeight: 700,
                fontSize: '46px',
                letterSpacing: '0.1em',
              }}
            >
              魔法拼音温室
            </text>

            {/* English botanical display subtitle */}
            <text
              x="210"
              y="108"
              textAnchor="middle"
              fill="#E5C886"
              style={{
                fontFamily: "'Fraunces', serif",
                fontWeight: 600,
                fontSize: '16px',
                letterSpacing: '0.24em',
              }}
            >
              PINYIN GREENHOUSE
            </text>

            <path
              d="M 52 122 L 368 122"
              stroke="#D8B36A"
              strokeWidth="1.2"
              strokeDasharray="3 5"
              opacity="0.65"
            />
          </svg>

          <p className="mb-8 max-w-md text-[15px] leading-relaxed text-[#E9E4D6]/90">
            培育声母晶芽与韵母灵花，在铜制蒸馏台调配四声魔药，用真实普通话发音为森林村民调制专属拼音药水。
          </p>

          {/* Menu actions (<= 3 actions total) */}
          <div className="flex flex-wrap items-center gap-4">
            <button
              type="button"
              data-start="true"
              onClick={onStart}
              className="inline-flex items-center gap-2.5 rounded-xl border border-[#F2D399]/60 bg-[#C96F4B] px-7 py-3.5 text-base font-semibold text-[#FFFDF8] shadow-[0_8px_24px_rgba(0,0,0,0.4)] transition-transform duration-150 hover:-translate-y-0.5 hover:bg-[#B85E3A] active:translate-y-0 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#F2D399] whitespace-nowrap shrink-0"
            >
              <Play className="h-5 w-5 fill-current" />
              <span>进入温室</span>
            </button>

            <button
              type="button"
              onClick={onOpenAnalysis}
              className="inline-flex items-center gap-2 rounded-xl border border-[#D8C9A8]/40 bg-[#1F2D21]/85 px-5 py-3.5 text-sm font-medium text-[#EFE6D2] transition-colors duration-150 hover:border-[#E5C886]/70 hover:bg-[#293B2C] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#E5C886] whitespace-nowrap shrink-0"
            >
              <BookOpen className="h-4 w-4 text-[#E5C886]" />
              <span>策划与资源复用分析</span>
            </button>
          </div>
        </div>
      </main>

      {/* Bottom flat borderless control hint */}
      <footer className="relative z-10 px-8 pb-5 sm:px-14 lg:px-20">
        <p className="max-w-[11.5rem] text-xs leading-relaxed tracking-wide text-[#DFD8C6]/75 sm:max-w-sm">
          Enter / Space 开始 · 点击花盆浇水收获 · 炼金台声韵拼读 · M 静音 · P 暂停
        </p>
      </footer>
    </div>
  );
};
