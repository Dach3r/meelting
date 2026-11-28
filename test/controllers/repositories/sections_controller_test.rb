require "test_helper"

class Repositories::SectionsControllerTest < ActionDispatch::IntegrationTest
  test "shows the push commands and an inline token button on an empty repository" do
    sign_in_as users(:one)
    get repository_path("ada", "meelting")

    assert_response :success
    assert_select "section#empty_repository h2", "Sube tu código"
    assert_select "#push_commands code", /git remote add origin http:\/\/www.example.com\/ada\/meelting.git/
    assert_select "#push_commands form[action=?] button", repository_push_token_path("ada", "meelting"), "Generar token"
  end

  test "shows the phase that builds each section once the repository has commits" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "README.md" => "# Meelting\n" })
      sign_in_as users(:one)

      phases = { repository_path("ada", "meelting") => 1 }
      { "issues" => 2, "pull_requests" => 2, "runs" => 3, "memory" => 4 }.each { |section, phase| phases[repository_section_path("ada", "meelting", section)] = phase }

      phases.each do |path, phase|
        get path

        assert_response :success
        assert_select "p", "Se construye en la Fase #{phase}."
        assert_select "#repository_license", 0
      end
    end
  end

  test "code shows meelting's own agpl license linking to the file" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "LICENSE" => Rails.root.join("LICENSE").read })
      sign_in_as users(:one)

      get repository_path("ada", "meelting")

      assert_select "#repository_license a[href=?]", repository_blob_path("ada", "meelting", "LICENSE"), "Licencia AGPL-3.0"
    end
  end

  test "code shows other for an unrecognized license file" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "LICENSE.md" => "All rights reserved.\n" })
      sign_in_as users(:one)

      get repository_path("ada", "meelting")

      assert_select "#repository_license a[href=?]", repository_blob_path("ada", "meelting", "LICENSE.md"), "Otra"
    end
  end

  test "scopes repositories to the current user" do
    sign_in_as users(:one)
    get repository_path("alan", "notes")

    assert_response :not_found
  end

  test "requires the username of the owner" do
    sign_in_as users(:one)
    get repository_path("alan", "meelting")

    assert_response :not_found
  end

  test "rejects unknown sections" do
    sign_in_as users(:one)

    assert_raises(ActionController::UrlGenerationError) { repository_section_path("ada", "meelting", "wiki") }
    assert_raises(ActionController::UrlGenerationError) { repository_section_path("ada", "meelting", "code") }
  end

  test "requires authentication" do
    get repository_path("ada", "meelting")

    assert_redirected_to new_session_path
  end
end
