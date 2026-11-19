import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { KeyRound } from "lucide-react";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../i18n/I18nProvider";
import { NavItem } from "./NavItem";

function renderItem(collapsed: boolean) {
  return render(
    <I18nProvider locale="en">
      <NavItem
        label="AI providers"
        icon={KeyRound}
        collapsed={collapsed}
        onSelect={() => {}}
      />
    </I18nProvider>,
  );
}

describe("NavItem", () => {
  it("shows the label in a flyout while hovering the collapsed item", async () => {
    const user = userEvent.setup();
    renderItem(true);
    const item = screen.getByRole("button", { name: "AI providers" });
    expect(screen.getAllByText("AI providers")).toHaveLength(1);

    await user.hover(item);
    expect(screen.getAllByText("AI providers")).toHaveLength(3);

    await user.unhover(item);
    expect(screen.getAllByText("AI providers")).toHaveLength(1);
  });

  it("shows no flyout while hovering the expanded item", async () => {
    const user = userEvent.setup();
    renderItem(false);

    await user.hover(screen.getByRole("button", { name: "AI providers" }));
    expect(screen.getAllByText("AI providers")).toHaveLength(1);
  });
});
