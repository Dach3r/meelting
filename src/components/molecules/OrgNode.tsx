import {
  useRef,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
} from "react";

const dragThresholdPx = 3;

type Props = {
  x: number;
  y: number;
  zoom: number;
  onMove: (x: number, y: number) => void;
  registerCard: (element: HTMLElement | null) => void;
  card: ReactNode;
  className: string;
  children?: ReactNode;
};

type Drag = {
  pointerX: number;
  pointerY: number;
  x: number;
  y: number;
  moving: boolean;
};

export function OrgNode({
  x,
  y,
  zoom,
  onMove,
  registerCard,
  card,
  className,
  children,
}: Props) {
  const drag = useRef<Drag | null>(null);
  const dragged = useRef(false);

  function start(event: PointerEvent<HTMLDivElement>) {
    if (
      event.button !== 0 ||
      (event.target as Element).closest("button:not([aria-pressed])")
    )
      return;
    event.stopPropagation();
    dragged.current = false;
    drag.current = {
      pointerX: event.clientX,
      pointerY: event.clientY,
      x,
      y,
      moving: false,
    };
  }

  function move(event: PointerEvent<HTMLDivElement>) {
    const current = drag.current;
    if (!current) return;
    const dx = event.clientX - current.pointerX;
    const dy = event.clientY - current.pointerY;
    if (!current.moving) {
      if (Math.hypot(dx, dy) < dragThresholdPx) return;
      current.moving = true;
      dragged.current = true;
      event.currentTarget.setPointerCapture?.(event.pointerId);
    }
    onMove(current.x + dx / zoom, current.y + dy / zoom);
  }

  function end() {
    drag.current = null;
  }

  function swallowClickAfterDrag(event: MouseEvent<HTMLDivElement>) {
    if (!dragged.current) return;
    dragged.current = false;
    event.stopPropagation();
  }

  return (
    <li
      className={className}
      style={{ transform: `translate(${x}px, ${y}px)` }}
    >
      <div
        ref={registerCard}
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={end}
        onClickCapture={swallowClickAfterDrag}
        className="touch-none"
      >
        {card}
      </div>
      {children}
    </li>
  );
}
