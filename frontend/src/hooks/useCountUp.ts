"use client";

import { useEffect, useRef } from "react";
import { gsap } from "gsap";

/**
 * GSAP-powered count-up animation for money displays.
 * Animates from 0 to `target` over `duration` seconds with `power2.out` easing.
 *
 * @param target The final value to count up to
 * @param deps Dependency array — animation re-fires when these change
 * @param duration Animation duration in seconds (default 2)
 * @returns A ref to attach to the element whose textContent should animate
 */
export function useCountUp(target: number, deps: any[] = [], duration = 2) {
  const ref = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!ref.current || target <= 0) return;

    const obj = { val: 0 };
    const tween = gsap.to(obj, {
      val: target,
      duration,
      ease: "power2.out",
      onUpdate: () => {
        if (ref.current) {
          ref.current.textContent = Math.floor(obj.val).toLocaleString();
        }
      },
    });

    return () => {
      tween.kill();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target, duration, ...deps]);

  return ref;
}
