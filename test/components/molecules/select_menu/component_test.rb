require "test_helper"

class Molecules::SelectMenu::ComponentTest < ViewComponent::TestCase
  OPTIONS = [["No license", ""], ["MIT License", 1], ["Apache License 2.0", 2]].freeze

  def builder(license_id: nil)
    ActionView::Helpers::FormBuilder.new(:repository, Repository.new(license_id: license_id), vc_test_controller.view_context, {})
  end

  def render_menu(**form_options)
    render_inline(Molecules::SelectMenu::Component.new(form: builder(**form_options), attribute: :license_id, label: "Add a license", icon: :scale, options: OPTIONS))
  end

  test "renders one radio per option under the legend" do
    menu = render_menu

    assert_equal "Add a license", menu.css("legend").text
    assert_equal ["", "1", "2"], menu.css("input[type=radio][name='repository[license_id]']").map { it["value"] }
  end

  test "shows the first option when nothing is selected" do
    menu = render_menu

    assert_equal "No license", menu.css("[data-select-menu-target=label]").text
    assert_equal "", menu.css("input[checked]").attr("value").value
  end

  test "checks and shows the selected option" do
    menu = render_menu(license_id: 1)

    assert_equal "MIT License", menu.css("[data-select-menu-target=label]").text
    assert_equal "1", menu.css("input[checked]").attr("value").value
  end

  test "without a label it is named by labelledby" do
    menu = render_inline(Molecules::SelectMenu::Component.new(form: builder, attribute: :license_id, label: nil, icon: :scale, options: OPTIONS, labelledby: "license_label"))

    assert_empty menu.css("legend")
    assert_equal "license_label", menu.css("fieldset").attr("aria-labelledby").value
  end

  test "compact uses the compact shell" do
    menu = render_inline(Molecules::SelectMenu::Component.new(form: builder, attribute: :license_id, label: nil, icon: :scale, options: OPTIONS, compact: true))

    assert_equal 1, menu.css("summary.nb-field-shell.nb-field-shell-compact").size
  end
end
