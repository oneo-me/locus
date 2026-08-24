import { describe, expect, test } from 'vitest';
import { colorSchemeForTheme } from './theme-provider';

describe('colorSchemeForTheme', () => {
  test('maps theme preferences to standard color-scheme metadata', () => {
    expect(colorSchemeForTheme('system')).toBe('light dark');
    expect(colorSchemeForTheme('light')).toBe('light');
    expect(colorSchemeForTheme('dark')).toBe('dark');
  });
});
