import { SPEAK_MAP, type ToneNumber } from '../data/pinyinResources';
import { getAudioUrls, getExampleChar } from './pinyinEngine';

class SoundEngine {
  private muted = false;
  private ctx: AudioContext | null = null;
  private activeAudio: HTMLAudioElement | null = null;

  public setMuted(muted: boolean): void {
    this.muted = muted;
    if (muted && this.activeAudio) {
      this.activeAudio.pause();
      this.activeAudio = null;
    }
  }

  public isMuted(): boolean {
    return this.muted;
  }

  private getAudioContext(): AudioContext | null {
    if (typeof window === 'undefined') return null;
    if (!this.ctx) {
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (AudioCtx) {
        this.ctx = new AudioCtx();
      }
    }
    if (this.ctx && this.ctx.state === 'suspended') {
      this.ctx.resume().catch(() => {});
    }
    return this.ctx;
  }

  /**
   * Plays real human pronunciation from flashtony2005/pinyin (bd/{syllable}{tone}.mp3 -> {key}.m4a),
   * gracefully falling back to browser zh-CN speechSynthesis when offline.
   */
  public async playPinyin(
    key: string,
    tone?: ToneNumber,
    options?: { slow?: boolean }
  ): Promise<void> {
    if (this.muted || !key || typeof window === 'undefined') return;

    if (this.activeAudio) {
      this.activeAudio.pause();
      this.activeAudio = null;
    }

    const urls = getAudioUrls(key, tone);
    const rate = options?.slow ? 0.72 : 1.0;

    for (const url of urls) {
      const played = await this.tryPlayUrl(url, rate);
      if (played) return;
    }

    // Fallback to browser zh-CN SpeechSynthesis (especially for 3-part syllables like jiā 家)
    try {
      if ('speechSynthesis' in window) {
        window.speechSynthesis.cancel();
        const fallbackChar = tone
          ? getExampleChar(key, tone).char
          : (SPEAK_MAP[key] ?? key);
        const utter = new SpeechSynthesisUtterance(fallbackChar);
        utter.lang = 'zh-CN';
        utter.rate = options?.slow ? 0.65 : 0.92;
        window.speechSynthesis.speak(utter);
      }
    } catch {
      // Ignore synthesis errors in headless environments
    }
  }

  private tryPlayUrl(url: string, playbackRate: number): Promise<boolean> {
    return new Promise((resolve) => {
      try {
        const audio = new Audio(url);
        audio.crossOrigin = 'anonymous';
        audio.playbackRate = playbackRate;
        this.activeAudio = audio;

        let settled = false;
        const finish = (ok: boolean) => {
          if (settled) return;
          settled = true;
          resolve(ok);
        };

        audio.addEventListener('error', () => finish(false), { once: true });
        const playPromise = audio.play();
        if (playPromise && typeof playPromise.then === 'function') {
          playPromise.then(() => finish(true)).catch(() => finish(false));
        } else {
          finish(true);
        }
      } catch {
        resolve(false);
      }
    });
  }

  public playSfx(kind: 'water' | 'harvest' | 'brew_ok' | 'brew_fail' | 'coin' | 'plant'): void {
    if (this.muted) return;
    const ctx = this.getAudioContext();
    if (!ctx) return;

    const now = ctx.currentTime;
    const playTone = (
      freq: number,
      endFreq: number,
      delay: number,
      duration: number,
      type: OscillatorType,
      gainPeak: number
    ) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = type;
      osc.frequency.setValueAtTime(freq, now + delay);
      osc.frequency.exponentialRampToValueAtTime(Math.max(40, endFreq), now + delay + duration);

      gain.gain.setValueAtTime(0.0001, now + delay);
      gain.gain.linearRampToValueAtTime(gainPeak, now + delay + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + delay + duration);

      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now + delay);
      osc.stop(now + delay + duration + 0.02);
    };

    switch (kind) {
      case 'water':
        playTone(460, 740, 0, 0.11, 'sine', 0.09);
        playTone(620, 920, 0.07, 0.1, 'sine', 0.06);
        break;
      case 'plant':
        playTone(280, 380, 0, 0.09, 'triangle', 0.08);
        break;
      case 'harvest':
        playTone(523.25, 523.25, 0, 0.12, 'triangle', 0.09);
        playTone(659.25, 659.25, 0.07, 0.14, 'triangle', 0.09);
        playTone(783.99, 783.99, 0.14, 0.22, 'sine', 0.09);
        break;
      case 'brew_ok':
        playTone(440, 440, 0, 0.13, 'sine', 0.08);
        playTone(554.37, 554.37, 0.07, 0.15, 'sine', 0.08);
        playTone(659.25, 659.25, 0.14, 0.2, 'triangle', 0.09);
        playTone(880, 880, 0.21, 0.3, 'sine', 0.07);
        break;
      case 'brew_fail':
        playTone(260, 150, 0, 0.18, 'sawtooth', 0.045);
        playTone(190, 115, 0.12, 0.22, 'triangle', 0.055);
        break;
      case 'coin':
        playTone(987.77, 987.77, 0, 0.08, 'sine', 0.08);
        playTone(1318.51, 1318.51, 0.07, 0.24, 'sine', 0.09);
        break;
    }
  }
}

export const soundEngine = new SoundEngine();
