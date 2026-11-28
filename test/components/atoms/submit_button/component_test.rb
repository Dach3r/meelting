require "test_helper"

class Atoms::SubmitButton::ComponentTest < ViewComponent::TestCase
  test "renders a submit button with the label" do
    button = render_inline(Atoms::SubmitButton::Component.new) { "Sign in" }.css("button[type=submit]")

    assert_includes button.text, "Sign in"
  end

  test "merges custom classes and passes html options through" do
    button = render_inline(Atoms::SubmitButton::Component.new(id: "go", class: "mt-2")) { "Go" }.css("button#go")

    assert_includes button.attr("class").value, "mt-2"
    assert_includes button.attr("class").value, "bg-forest"
  end
end
