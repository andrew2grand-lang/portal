import { motion } from 'motion/react'
import type { HTMLMotionProps, Variants } from 'motion/react'
import type { ReactNode } from 'react'
import { spring } from '../../styles/motion'
import s from './Key.module.css'

export type KeyVariant = 'plain' | 'ink' | 'lit' | 'danger'
export type KeyTone = 'blue' | 'orange' | 'dual'
export type KeySize = 'sm' | 'md' | 'lg'

export interface KeyProps extends Omit<HTMLMotionProps<'button'>, 'children'> {
  /** plain: neutral cap. ink: high-contrast primary. lit: softly backlit (selected, or a signature action). danger: a plain cap in red, for what removes things everywhere (Clear Shelf) */
  variant?: KeyVariant
  /** Backlight colour when lit: blue = sending end, orange = receiving end, dual = both */
  tone?: KeyTone
  size?: KeySize
  icon?: ReactNode
  /** Indicator LED in the cap corner, visible when lit */
  led?: boolean
  children?: ReactNode
}

/*
  Layer stack (bottom to top), each its own element so richer key
  animations can target them later without changing the API:
    glow  backlight spilling onto the surface (lit only)
    drop  soft cast shadow, compresses on press
    base  the key's skirt, the "height" you see below the cap
    cap   travels down onto the base when pressed
      face  bevelled top surface, label + icon + LED
*/
const capVariants: Variants = {
  rest: { y: 0, transition: spring.release },
  pressed: { y: 2, transition: spring.press },
}
const dropVariants: Variants = {
  rest: { opacity: 1, transition: spring.release },
  pressed: { opacity: 0.35, transition: spring.press },
}

export function Key({
  variant = 'plain',
  tone = 'blue',
  size = 'md',
  icon,
  led = false,
  children,
  className,
  type = 'button',
  disabled,
  ...rest
}: KeyProps) {
  return (
    <motion.button
      type={type}
      className={className ? `${s.key} ${className}` : s.key}
      data-variant={variant}
      data-tone={tone}
      data-size={size}
      data-icon-only={children ? undefined : ''}
      disabled={disabled}
      initial={false}
      animate="rest"
      whileTap={disabled ? undefined : 'pressed'}
      {...rest}
    >
      <span className={s.glow} aria-hidden />
      <motion.span className={s.drop} variants={dropVariants} aria-hidden />
      <span className={s.base} aria-hidden />
      <motion.span className={s.cap} variants={capVariants}>
        <span className={s.face}>
          {icon && (
            <span className={s.icon} aria-hidden>
              {icon}
            </span>
          )}
          {children && <span className={s.label}>{children}</span>}
          {led && <span className={s.led} aria-hidden />}
        </span>
      </motion.span>
    </motion.button>
  )
}
