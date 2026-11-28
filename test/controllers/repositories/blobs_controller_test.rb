require "test_helper"

class Repositories::BlobsControllerTest < ActionDispatch::IntegrationTest
  setup do
    License.sync_catalog
    sign_in_as users(:one)
  end

  test "shows a recognized license file with its summary" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "LICENSE" => License.find_by!(key: "mit").render(user: users(:one)) })

      get repository_blob_path("meelting", "LICENSE")

      assert_response :success
      assert_select "#license_summary h2", "MIT License"
      assert_select "#license_summary [data-group=permissions] li", "Uso comercial"
      assert_select "#license_summary [data-group=conditions] li", "Aviso de licencia y copyright"
      assert_select "#license_summary [data-group=limitations] li", "Responsabilidad"
      assert_select "pre#blob", /Copyright \(c\) \d{4} Ada Lovelace/
    end
  end

  test "shows other files as plain text without a summary" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "LICENSE" => Rails.root.join("LICENSE").read, "docs/notes.txt" => "<b>hi</b>\n" })

      get repository_blob_path("meelting", "docs/notes.txt")

      assert_response :success
      assert_select "#license_summary", 0
      assert_select "pre#blob", "<b>hi</b>\n"
    end
  end

  test "an unrecognized license file has no summary" do
    with_git_root do
      create_bare_repository(repositories(:meelting), files: { "LICENSE" => "All rights reserved.\n" })

      get repository_blob_path("meelting", "LICENSE")

      assert_select "#license_summary", 0
      assert_select "pre#blob"
    end
  end

  test "missing files, directories and empty repositories are not found" do
    with_git_root do
      get repository_blob_path("meelting", "LICENSE")
      assert_response :not_found

      create_bare_repository(repositories(:meelting), files: { "docs/notes.txt" => "x" })
      get repository_blob_path("meelting", "nope.txt")
      assert_response :not_found
      get repository_blob_path("meelting", "docs")
      assert_response :not_found
    end
  end

  test "scopes repositories to the current user" do
    get repository_blob_path("notes", "LICENSE")

    assert_response :not_found
  end
end
