require "test_helper"

class Molecules::SettingRow::ComponentTest < ViewComponent::TestCase
  test "binds the label to the control and renders the content beside it" do
    row = render_inline(Molecules::SettingRow::Component.new(label: "Name", label_for: "repository_name")) { "control" }

    assert_equal "Name", row.css("label[for=repository_name]").text
    assert_includes row.text, "control"
  end

  test "renders a span with an id when there is no single control" do
    row = render_inline(Molecules::SettingRow::Component.new(label: "Visibility", id: "visibility_label"))

    assert_equal "Visibility", row.css("span#visibility_label").text
    assert_empty row.css("label")
  end
end
