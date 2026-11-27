require "test_helper"

class Atoms::PillButton::ComponentTest < ViewComponent::TestCase
  test "renders a plain button with the variant classes" do
    button = render_inline(Atoms::PillButton::Component.new(variant: :primary)) { "Go" }.css("button")

    assert_equal "Go", button.text
    assert_equal "button", button.attr("type").value
    assert_includes button.attr("class").value, "bg-sun-400"
  end

  test "defaults to the gray variant" do
    button = render_inline(Atoms::PillButton::Component.new) { "Go" }.css("button")

    assert_includes button.attr("class").value, "bg-graphite-50"
  end

  test "danger variant uses coral text" do
    button = render_inline(Atoms::PillButton::Component.new(variant: :danger)) { "Reject" }.css("button")

    assert_includes button.attr("class").value, "text-coral-700"
  end

  test "merges custom classes" do
    button = render_inline(Atoms::PillButton::Component.new(class: "w-full")) { "Go" }.css("button")

    assert_includes button.attr("class").value, "w-full"
  end
end
