require "test_helper"

class Atoms::LinkButton::ComponentTest < ViewComponent::TestCase
  test "renders a link with the variant and size classes" do
    link = render_inline(Atoms::LinkButton::Component.new(href: "/go", variant: :signal, size: :lg)) { "Go" }.css("a[href='/go']")

    assert_equal "Go", link.text
    assert_includes link.attr("class").value, "bg-signal"
    assert_includes link.attr("class").value, "px-5"
  end

  test "defaults to the primary variant" do
    link = render_inline(Atoms::LinkButton::Component.new(href: "/go")) { "Go" }.css("a")

    assert_includes link.attr("class").value, "bg-accent"
  end

  test "raises on an unknown variant" do
    assert_raises(KeyError) do
      render_inline(Atoms::LinkButton::Component.new(href: "/go", variant: :nope)) { "Go" }
    end
  end
end
