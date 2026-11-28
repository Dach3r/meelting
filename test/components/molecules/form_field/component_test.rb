require "test_helper"

class Molecules::FormField::ComponentTest < ViewComponent::TestCase
  def builder
    ActionView::Helpers::FormBuilder.new(:user, User.new, vc_test_controller.view_context, {})
  end

  test "renders a label bound to a required input" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user))

    assert_equal "Username", field.css("label[for='user_username']").text
    assert_equal 1, field.css("input#user_username[type=text][required].nb-field-shell-input").size
  end

  test "disables autocomplete by default" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user))

    assert_equal "off", field.css("input").attr("autocomplete").value
  end

  test "type selects the input kind" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :password, label: "Password", icon: :lock, type: :password))

    assert_equal 1, field.css("input[type=password]").size
  end

  test "passes input options through" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :email_address, label: "Email", icon: :mail, type: :email, placeholder: "you@company.com", autocomplete: "email"))

    assert_equal "you@company.com", field.css("input").attr("placeholder").value
    assert_equal "email", field.css("input").attr("autocomplete").value
  end

  test "renders the icon beside the input" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user))

    assert_equal 1, field.css(".nb-field-shell .nb-field-shell-icon svg").size
  end

  test "renders the status below the input" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user)) do |component|
      component.with_status { "Checking" }
    end

    assert_equal "Checking", field.css("div.grid").first.children.last.text.strip
  end

  test "renders the label aside next to the label" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :password, label: "Password", icon: :lock, type: :password)) do |component|
      component.with_label_aside { "Forgot?" }
    end

    assert_equal "Password Forgot?", field.css("label").first.parent.text.squish
  end

  test "text area type renders a textarea in an area shell" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user, type: :text_area, required: false))

    assert_equal 1, field.css(".nb-field-shell.nb-field-shell-area textarea.nb-field-shell-input:not([required])").size
  end

  test "control replaces the default input" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: "Username", icon: :user)) do |component|
      component.with_control { "<select class='custom'></select>".html_safe }
    end

    assert_equal 1, field.css(".nb-field-shell select.custom").size
    assert_empty field.css("input")
  end

  test "without a label it renders only the input" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: nil, icon: :user))

    assert_empty field.css("label")
    assert_equal 1, field.css("input#user_username").size
  end

  test "compact uses the compact shell" do
    field = render_inline(Molecules::FormField::Component.new(form: builder, attribute: :username, label: nil, icon: :user, compact: true))

    assert_equal 1, field.css(".nb-field-shell.nb-field-shell-compact input#user_username").size
  end
end
