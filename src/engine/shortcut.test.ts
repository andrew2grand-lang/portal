import { describe, expect, it } from 'vitest'
import { acceleratorFrom, shortcutLabel } from './shortcut'

const key = (over: Partial<KeyboardEvent>) =>
  ({ key: 'c', code: 'KeyC', ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...over }) as KeyboardEvent

describe('the Send clipboard shortcut', () => {
  it('reads like each system writes it', () => {
    expect(shortcutLabel('Control+Alt+Super+C', 'mac')).toBe('⌃⌥⌘C')
    expect(shortcutLabel('Control+Alt+Shift+C', 'win')).toBe('Ctrl+Alt+Shift+C')
    expect(shortcutLabel('Control+Shift+Digit5', 'mac')).toBe('⌃⇧5')
    expect(shortcutLabel('', 'mac')).toBe('')
  })
  it('turns a key press into a shortcut', () => {
    expect(acceleratorFrom(key({ ctrlKey: true, altKey: true, metaKey: true }), 'mac')).toBe('Control+Alt+Super+C')
    expect(acceleratorFrom(key({ ctrlKey: true, altKey: true, shiftKey: true }), 'win')).toBe('Control+Alt+Shift+C')
    expect(acceleratorFrom(key({ code: 'Digit5', key: '5', ctrlKey: true, shiftKey: true }), 'win')).toBe('Control+Shift+Digit5')
  })
  it('needs a modifier and a real key', () => {
    expect(acceleratorFrom(key({}), 'mac')).toBeNull()
    expect(acceleratorFrom(key({ shiftKey: true }), 'mac')).toBeNull()
    expect(acceleratorFrom(key({ key: 'Control', code: 'ControlLeft', ctrlKey: true }), 'win')).toBeNull()
  })
})
