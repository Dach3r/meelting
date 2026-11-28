require "test_helper"

class SidebarTest < ActionDispatch::IntegrationTest
  test "authenticated pages render the sidebar in order" do
    sign_in_as users(:one)
    get repositories_path

    assert_select "nav[aria-label='Navegación del espacio de trabajo']" do
      assert_select "a[href=?]", root_path
      assert_select "a[href=?]", runs_path
      assert_select "button[aria-expanded]", text: /Meelting/
    end
  end

  test "repository sections link to their pages" do
    sign_in_as users(:one)
    get root_path

    %w[code issues pull_requests runs memory].each do |section|
      assert_select "#sidebar-repository-meelting a[href=?]", repository_section_path("meelting", section)
    end
  end

  test "renders the rail by default and the expanded sidebar when its cookie says so" do
    sign_in_as users(:one)
    get root_path
    assert_select "body[data-sidebar-expanded]", count: 0

    cookies[ApplicationHelper::SIDEBAR_COOKIE] = "1"
    get root_path
    assert_select "body[data-sidebar-expanded]"
  end

  test "the sidebar is absent for guests" do
    get unauthenticated_root_path

    assert_select "nav[aria-label='Navegación del espacio de trabajo']", count: 0
  end

  test "preview shows counters and mirror dots in development" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "inbox")
    end

    assert_select "a[href=?] span[aria-label=?]", root_path, "3 pendientes"
    assert_select "a[href=?] [role=img][aria-label=?]", runs_path, "1 runs en curso"
    assert_select "[role=img][aria-label=?]", "Mirror al día"
  end

  test "preview empty shows a failing mirror dot" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "empty")
    end

    assert_select "[role=img][aria-label=?]", "Mirror fallando"
  end

  test "preview first_run does not label real data as preview" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "first_run")
    end

    assert_select "*", text: /Datos de ejemplo/, count: 0
  end
end
