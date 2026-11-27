require "test_helper"

class Atoms::SubmitButton::ComponentTest < ViewComponent::TestCase
  test "renders a submit button with the label and an arrow" do
    button = render_inline(Atoms::SubmitButton::Component.new) { "Sign in" }.css("button[type=submit]")

    assert_includes button.text, "Sign in"
    assert_equal 1, button.css("svg").size
  end

  test "merges custom classes and passes html options through" do
    button = render_inline(Atoms::SubmitButton::Component.new(id: "go", class: "mt-2")) { "Go" }.css("button#go")

    assert_includes button.attr("class").value, "mt-2"
    assert_includes button.attr("class").value, "bg-sun-400"
  end
end
