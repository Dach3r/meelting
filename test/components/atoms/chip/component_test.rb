require "test_helper"

class Atoms::Chip::ComponentTest < ViewComponent::TestCase
  test "renders the content as a dashed chip" do
    chip = render_inline(Atoms::Chip::Component.new) { "New" }.css("span")

    assert_equal "New", chip.text
    assert_includes chip.attr("class").value, "nb-chip"
  end

  test "passes html options through" do
    chip = render_inline(Atoms::Chip::Component.new(id: "chip")) { "x" }

    assert_equal 1, chip.css("span#chip").size
  end

  test "merges custom classes with the base classes" do
    chip = render_inline(Atoms::Chip::Component.new(class: "text-sm")) { "x" }.css("span")

    assert_includes chip.attr("class").value, "text-sm"
    assert_includes chip.attr("class").value, "nb-chip"
  end
end
