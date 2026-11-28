require "test_helper"

class Molecules::Notice::ComponentTest < ViewComponent::TestCase
  test "alert uses the alert role and danger tone" do
    notice = render_inline(Molecules::Notice::Component.new(tone: :alert)) { "Nope" }.css("div")

    assert_equal "Nope", notice.text
    assert_equal "alert", notice.attr("role").value
    assert_includes notice.attr("class").value, "text-danger"
  end

  test "notice uses the status role and lime tone" do
    notice = render_inline(Molecules::Notice::Component.new(tone: :notice)) { "Done" }.css("div")

    assert_equal "status", notice.attr("role").value
    assert_includes notice.attr("class").value, "text-forest"
  end

  test "passes id and merges classes" do
    notice = render_inline(Molecules::Notice::Component.new(tone: :alert, id: "alert", class: "mb-0")) { "x" }.css("div#alert")

    assert_includes notice.attr("class").value, "mb-0"
  end
end
