require "test_helper"

class Repositories::SectionsControllerTest < ActionDispatch::IntegrationTest
  test "shows the phase that builds each section" do
    sign_in_as users(:one)

    { "code" => 1, "issues" => 2, "pull_requests" => 2, "runs" => 3, "memory" => 4 }.each do |section, phase|
      get repository_section_path("meelting", section)

      assert_response :success
      assert_select "p", "Se construye en la Fase #{phase}."
    end
  end

  test "scopes repositories to the current user" do
    sign_in_as users(:one)
    get repository_section_path("notes", "code")

    assert_response :not_found
  end

  test "rejects unknown sections" do
    sign_in_as users(:one)

    assert_raises(ActionController::UrlGenerationError) { repository_section_path("meelting", "wiki") }
  end

  test "requires authentication" do
    get repository_section_path("meelting", "code")

    assert_redirected_to new_session_path
  end
end
