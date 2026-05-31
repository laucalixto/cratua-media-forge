// ── Cratua Media Forge — JS Unit Tests ──
// Run: node --test crates/mediaforge-tauri/ui/test.js

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

// ── Functions extracted for testing ──

function even(n) {
    n = parseInt(n);
    if (isNaN(n)) return 1920;
    return n % 2 !== 0 ? n + 1 : n;
}

function normPath(p) {
    return (p || '').replace(/\\/g, '/');
}

function parseVideoFilter(s) {
    if (s === 'HFlip') return { HFlip: null };
    if (s === 'VFlip') return { VFlip: null };
    if (s === 'Denoise') return { Denoise: null };
    if (s === 'Grayscale') return { Grayscale: null };
    const m = s.match(/^Rotate\((\d+)\)$/);
    if (m) return { Rotate: parseInt(m[1]) };
    const b = s.match(/^Brightness\(([0-9.+\-]+)\)$/);
    if (b) return { Brightness: parseFloat(b[1]) };
    const c = s.match(/^Contrast\(([0-9.+\-]+)\)$/);
    if (c) return { Contrast: parseFloat(c[1]) };
    const st = s.match(/^Saturation\(([0-9.+\-]+)\)$/);
    if (st) return { Saturation: parseFloat(st[1]) };
    return { HFlip: null };
}

function parseAudioFilter(s) {
    if (s === 'Loudnorm') return { Loudnorm: null };
    const v = s.match(/^Volume\(([0-9.]+)\)$/);
    if (v) return { Volume: parseFloat(v[1]) };
    const h = s.match(/^Highpass\((\d+)\)$/);
    if (h) return { Highpass: parseInt(h[1]) };
    const l = s.match(/^Lowpass\((\d+)\)$/);
    if (l) return { Lowpass: parseInt(l[1]) };
    return { Loudnorm: null };
}

// ── Tests ──

describe('even()', () => {
    it('360 stays 360', () => assert.equal(even(360), 360));
    it('361 rounds to 362', () => assert.equal(even(361), 362));
    it('NaN returns default 1920', () => assert.equal(even('abc'), 1920));
    it('0 stays 0', () => assert.equal(even(0), 0));
    it('negative rounds up', () => assert.equal(even(-1), 0));
});

describe('normPath()', () => {
    it('backslash to slash', () => assert.equal(normPath('W:\\pasta\\file.mp4'), 'W:/pasta/file.mp4'));
    it('keeps slash', () => assert.equal(normPath('/home/user/file.mp4'), '/home/user/file.mp4'));
    it('mixed', () => assert.equal(normPath('C:\\Users\\test/file.mp4'), 'C:/Users/test/file.mp4'));
    it('null returns empty', () => assert.equal(normPath(null), ''));
    it('empty returns empty', () => assert.equal(normPath(''), ''));
});

describe('parseVideoFilter()', () => {
    it('HFlip', () => assert.deepEqual(parseVideoFilter('HFlip'), { HFlip: null }));
    it('VFlip', () => assert.deepEqual(parseVideoFilter('VFlip'), { VFlip: null }));
    it('Denoise', () => assert.deepEqual(parseVideoFilter('Denoise'), { Denoise: null }));
    it('Grayscale', () => assert.deepEqual(parseVideoFilter('Grayscale'), { Grayscale: null }));
    it('Rotate(90)', () => assert.deepEqual(parseVideoFilter('Rotate(90)'), { Rotate: 90 }));
    it('Rotate(180)', () => assert.deepEqual(parseVideoFilter('Rotate(180)'), { Rotate: 180 }));
    it('Brightness(0.5)', () => assert.deepEqual(parseVideoFilter('Brightness(0.5)'), { Brightness: 0.5 }));
    it('Contrast(1.5)', () => assert.deepEqual(parseVideoFilter('Contrast(1.5)'), { Contrast: 1.5 }));
    it('Saturation(2.0)', () => assert.deepEqual(parseVideoFilter('Saturation(2.0)'), { Saturation: 2.0 }));
    it('unknown returns fallback', () => assert.deepEqual(parseVideoFilter('Unknown'), { HFlip: null }));
});

describe('parseAudioFilter()', () => {
    it('Loudnorm', () => assert.deepEqual(parseAudioFilter('Loudnorm'), { Loudnorm: null }));
    it('Volume(2.0)', () => assert.deepEqual(parseAudioFilter('Volume(2.0)'), { Volume: 2.0 }));
    it('Volume(0.5)', () => assert.deepEqual(parseAudioFilter('Volume(0.5)'), { Volume: 0.5 }));
    it('Highpass(100)', () => assert.deepEqual(parseAudioFilter('Highpass(100)'), { Highpass: 100 }));
    it('Lowpass(3000)', () => assert.deepEqual(parseAudioFilter('Lowpass(3000)'), { Lowpass: 3000 }));
    it('unknown returns fallback', () => assert.deepEqual(parseAudioFilter('Unknown'), { Loudnorm: null }));
});

// ── New functions from code review ──

function clamp(v, min, max, fallback) {
    const n = parseInt(v);
    return isNaN(n) ? fallback : Math.max(min, Math.min(max, n));
}

describe('clamp()', () => {
    it('within range', () => assert.equal(clamp(50, 0, 100, 0), 50));
    it('below min', () => assert.equal(clamp(-5, 0, 100, 0), 0));
    it('above max', () => assert.equal(clamp(999, 0, 100, 0), 100));
    it('NaN returns fallback', () => assert.equal(clamp('abc', 0, 100, 42), 42));
    it('empty string returns fallback', () => assert.equal(clamp('', 0, 100, 42), 42));
});

// ── Error sanitization (encodeURIComponent roundtrip) ──

describe('error sanitization', () => {
    it('roundtrips encode/decode', () => {
        const err = 'Error: can\'t "parse" this & that <tag>';
        const safe = encodeURIComponent(err);
        // encodeURIComponent escapes ", <, >, & but not '
        assert(!safe.includes('"'));
        assert(!safe.includes('<'));
        assert(!safe.includes('>'));
        assert.equal(decodeURIComponent(safe), err);
    });
    it('handles null/undefined', () => {
        const safe = encodeURIComponent(null || 'unknown');
        assert.equal(decodeURIComponent(safe), 'unknown');
    });
    it('handles ffmpeg error with quotes', () => {
        const err = 'ffmpeg exited with code 1\nCommand: ffmpeg -i "in.mp4"';
        const safe = encodeURIComponent(err);
        const decoded = decodeURIComponent(safe);
        assert.equal(decoded, err);
        // Safe for data attribute
        assert(!safe.includes('"'));
    });
});

// ── Filter label helpers ──

function vfLabel(f) {
    if (typeof f === 'string') return f;
    for (const k of ['HFlip', 'VFlip', 'Denoise', 'Grayscale', 'Rotate', 'Brightness', 'Contrast', 'Saturation']) {
        if (k in f) {
            switch (k) {
                case 'HFlip': return 'Flip H';
                case 'VFlip': return 'Flip V';
                case 'Denoise': return 'Denoise';
                case 'Grayscale': return 'Grayscale';
                case 'Rotate': return 'Rotate ' + f.Rotate + '\u00b0';
                case 'Brightness': return 'Bright ' + f.Brightness;
                case 'Contrast': return 'Contrast ' + f.Contrast;
                case 'Saturation': return 'Sat ' + f.Saturation;
            }
        }
    }
    return JSON.stringify(f);
}

function afLabel(f) {
    if (typeof f === 'string') return f;
    for (const k of ['Loudnorm', 'Volume', 'Highpass', 'Lowpass']) {
        if (k in f) {
            switch (k) {
                case 'Loudnorm': return 'Loudnorm';
                case 'Volume': return 'Vol ' + f.Volume + 'x';
                case 'Highpass': return 'HP ' + f.Highpass + 'Hz';
                case 'Lowpass': return 'LP ' + f.Lowpass + 'Hz';
            }
        }
    }
    return JSON.stringify(f);
}

describe('vfLabel()', () => {
    it('HFlip', () => assert.equal(vfLabel({ HFlip: null }), 'Flip H'));
    it('Rotate 90', () => assert.equal(vfLabel({ Rotate: 90 }), 'Rotate 90°'));
    it('Brightness', () => assert.equal(vfLabel({ Brightness: 0.5 }), 'Bright 0.5'));
    it('string pass-through', () => assert.equal(vfLabel('custom'), 'custom'));
});

describe('afLabel()', () => {
    it('Loudnorm', () => assert.equal(afLabel({ Loudnorm: null }), 'Loudnorm'));
    it('Volume 2x', () => assert.equal(afLabel({ Volume: 2.0 }), 'Vol 2x'));
    it('Highpass 100Hz', () => assert.equal(afLabel({ Highpass: 100 }), 'HP 100Hz'));
    it('string pass-through', () => assert.equal(afLabel('custom'), 'custom'));
});
