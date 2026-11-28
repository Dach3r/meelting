require "test_helper"

class Atoms::LinkButton::ComponentTest < ViewComponent::TestCase
  test "renders a link with the variant and size classes" do
    link = render_inline(Atoms::LinkButton::Component.new(href: "/go", variant: :secondary, size: :lg)) { "Go" }.css("a[href='/go']")

    assert_equal "Go", link.text
    assert_includes link.attr("class").value, "bg-white"
    assert_includes link.attr("class").value, "h-14"
  end

  test "defaults to the forest primary variant" do
    link = render_inline(Atoms::LinkButton::Component.new(href: "/go")) { "Go" }.css("a")

    assert_includes link.attr("class").value, "bg-forest"
    assert_includes link.attr("class").value, "text-lime"
  end

  test "secondary variant is outlined" do
    link = render_inline(Atoms::LinkButton::Component.new(href: "/go", variant: :secondary)) { "Go" }.css("a")

    assert_includes link.attr("class").value, "border-ink"
    assert_includes link.attr("class").value, "bg-white"
  end

  test "raises on an unknown variant" do
    assert_raises(KeyError) do
      render_inline(Atoms::LinkButton::Component.new(href: "/go", variant: :nope)) { "Go" }
    end
  end
end
