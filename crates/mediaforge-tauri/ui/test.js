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
