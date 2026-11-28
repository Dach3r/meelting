require "test_helper"

class Organisms::Navbar::ComponentTest < ViewComponent::TestCase
  test "renders only the sign in link when unauthenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: false))

    assert_equal 1, nav.css("a[href='/session/new']").size
    assert_empty nav.css("a[href='/registration/new']")
    assert_empty nav.css("form[action='/session']")
    assert_empty nav.css("a[href='/repositories']")
  end

  test "renders search and notifications when authenticated, the user menu lives in the sidebar" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: true))

    assert_equal 1, nav.css("button[aria-label='#{I18n.t("navbar.search_label")}']").size
    assert_empty nav.css("details")
    assert_empty nav.css("form[action='/session']")
    assert_empty nav.css("a[href='/session/new']")
  end

  test "shows the brand only to guests, the sidebar owns it once signed in" do
    guest = render_inline(Organisms::Navbar::Component.new(authenticated: false))
    member = render_inline(Organisms::Navbar::Component.new(authenticated: true))

    assert_equal ApplicationHelper::APP_NAME, guest.css("a[href='/']").attr("aria-label").value
    assert_empty member.css("a[href='/']")
  end

  test "renders notifications bell only when authenticated" do
    label = I18n.t("navbar.notifications_label")

    assert_equal 1, render_inline(Organisms::Navbar::Component.new(authenticated: true)).css("button[aria-label='#{label}']").size
    assert_empty render_inline(Organisms::Navbar::Component.new(authenticated: false)).css("button[aria-label='#{label}']")
  end

  test "renders the new repository link next to the bell only when authenticated" do
    member = render_inline(Organisms::Navbar::Component.new(authenticated: true))
    guest = render_inline(Organisms::Navbar::Component.new(authenticated: false))

    link = member.css("nav button[aria-label='#{I18n.t("navbar.notifications_label")}'] + a[href='/repositories/new']")
    assert_equal I18n.t("navbar.new_repository_label"), link.attr("aria-label").value
    assert_empty guest.css("a[href='/repositories/new']")
  end

  test "sign in link has the same height as the signed in controls" do
    link = render_inline(Organisms::Navbar::Component.new(authenticated: false)).css("a[href='/session/new']")

    assert_includes link.attr("class").value, "h-14"
  end
end
