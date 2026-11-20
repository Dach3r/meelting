import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Switch } from "./Switch";

describe("Switch", () => {
  it("exposes its state and label as a switch", () => {
    render(<Switch label="Sound" checked onChange={() => {}} />);
    expect(screen.getByRole("switch", { name: "Sound" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("asks for the opposite state when clicked", async () => {
    const onChange = vi.fn();
    render(<Switch label="Sound" checked={false} onChange={onChange} />);
    await userEvent.click(screen.getByRole("switch", { name: "Sound" }));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("does nothing when disabled", async () => {
    const onChange = vi.fn();
    render(
      <Switch label="Sound" checked={false} onChange={onChange} disabled />,
    );
    await userEvent.click(screen.getByRole("switch", { name: "Sound" }));
    expect(onChange).not.toHaveBeenCalled();
  });
});
