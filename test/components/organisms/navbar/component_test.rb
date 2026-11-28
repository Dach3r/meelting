require "test_helper"

class Organisms::Navbar::ComponentTest < ViewComponent::TestCase
  test "renders only the sign in link when unauthenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: false))

    assert_equal 1, nav.css("a[href='/session/new']").size
    assert_empty nav.css("a[href='/registration/new']")
    assert_empty nav.css("form[action='/session']")
    assert_empty nav.css("a[href='/repositories']")
  end

  test "renders navigation and sign out button when authenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: true, user: users(:one)))

    assert_equal 1, nav.css("a[href='/repositories']").size
    assert_equal 1, nav.css("a[href='/access_tokens']").size
    assert_equal 1, nav.css("form[action='/session'] button").size
    assert_empty nav.css("a[href='/session/new']")
  end

  test "shows the brand to guests and a workspace pill hidden while the sidebar is expanded" do
    guest = render_inline(Organisms::Navbar::Component.new(authenticated: false))
    member = render_inline(Organisms::Navbar::Component.new(authenticated: true, user: users(:one)))

    assert_equal ApplicationHelper::CONFIG[:name], guest.css("a[href='/']").text.strip
    pill = member.css("a[href='/']").first
    assert_equal ApplicationHelper::CONFIG[:name], pill["aria-label"]
    assert_includes pill.text, I18n.t("navbar.workspace")
    assert_includes pill["class"], "lg:group-data-sidebar-expanded/shell:hidden"
  end

  test "renders notifications bell only when authenticated" do
    label = I18n.t("navbar.notifications_label")

    assert_equal 1, render_inline(Organisms::Navbar::Component.new(authenticated: true, user: users(:one))).css("button[aria-label='#{label}']").size
    assert_empty render_inline(Organisms::Navbar::Component.new(authenticated: false)).css("button[aria-label='#{label}']")
  end

  test "sign in link has the same height as the signed in controls" do
    link = render_inline(Organisms::Navbar::Component.new(authenticated: false)).css("a[href='/session/new']")

    assert_includes link.attr("class").value, "h-14"
  end
end
