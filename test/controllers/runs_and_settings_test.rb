require "test_helper"

class RunsAndSettingsTest < ActionDispatch::IntegrationTest
  test "runs page says which phase builds it" do
    sign_in_as users(:one)
    get runs_path

    assert_select "h1", "Runs"
    assert_select "p", "Se construye en la Fase 3."
  end

  test "settings links to access tokens" do
    sign_in_as users(:one)
    get settings_path

    assert_select "a[href=?]", access_tokens_path
  end

  test "both require authentication" do
    get runs_path
    assert_redirected_to new_session_path

    get settings_path
    assert_redirected_to new_session_path
  end
end
