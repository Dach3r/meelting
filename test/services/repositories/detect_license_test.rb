require "test_helper"

class Repositories::DetectLicenseTest < ActiveSupport::TestCase
  setup do
    License.sync_catalog
  end

  def push_commit(repository, files)
    Dir.mktmpdir do |work_|
      git = ->(*args_) { system("git", "-C", work_, *args_, exception: true, out: File::NULL, err: File::NULL) }
      git.call("clone", "-q", repository.git_path.to_s, ".")
      files.each { |path_, content_| File.write(File.join(work_, path_), content_) }
      git.call("add", "-A")
      git.call("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "-m", "Change license")
      git.call("push", "-q", "origin", "HEAD:#{repository.default_branch}")
    end
  end

  def with_memory_cache
    original = Rails.cache
    Rails.cache = ActiveSupport::Cache::MemoryStore.new
    yield
  ensure
    Rails.cache = original
  end

  test "every selectable license is detected back as itself" do
    with_git_root do
      License::SELECTABLE_KEYS.each do |key_|
        repository = users(:one).repositories.new(name: "Repo #{key_}", license: License.find_by!(key: key_))
        assert Repositories::Create.call(repository), key_

        result = Repositories::DetectLicense.call(repository)
        assert_equal key_, result&.key, key_
        assert_equal "LICENSE", result.path
      end
    end
  end

  test "detects meelting's own agpl license pushed to the repository" do
    with_git_root do
      repository = repositories(:meelting)
      create_bare_repository(repository, files: { "LICENSE" => Rails.root.join("LICENSE").read, "README.md" => "# Meelting\n" })

      result = Repositories::DetectLicense.call(repository)

      assert_equal "agpl-3.0", result.key
      assert_equal "AGPL-3.0", result.spdx_id
    end
  end

  test "a push that changes the license updates the result" do
    with_memory_cache do
      with_git_root do
        repository = repositories(:meelting)
        create_bare_repository(repository, files: { "LICENSE" => Rails.root.join("LICENSE").read })
        assert_equal "agpl-3.0", Repositories::DetectLicense.call(repository).key

        push_commit(repository, "LICENSE" => License.find_by!(key: "mit").render(user: users(:one)))

        assert_equal "mit", Repositories::DetectLicense.call(repository).key
      end
    end
  end

  test "finds common license file names" do
    with_git_root do
      repository = repositories(:meelting)
      create_bare_repository(repository, files: { "COPYING" => License.find_by!(key: "gpl-3.0").template })

      result = Repositories::DetectLicense.call(repository)

      assert_equal "gpl-3.0", result.key
      assert_equal "COPYING", result.path
    end
  end

  test "an unrecognized license file is other" do
    with_git_root do
      repository = repositories(:meelting)
      create_bare_repository(repository, files: { "LICENSE" => "All rights reserved.\n" })

      result = Repositories::DetectLicense.call(repository)

      assert result.other?
      assert_nil result.spdx_id
    end
  end

  test "no license file or no commits gives nothing" do
    with_git_root do
      repository = repositories(:meelting)
      assert_nil Repositories::DetectLicense.call(repository)

      create_bare_repository(repository, files: { "README.md" => "# Meelting\n" })
      assert_nil Repositories::DetectLicense.call(repository)
    end
  end

  test "exposes the rules of the detected license" do
    with_git_root do
      repository = repositories(:meelting)
      create_bare_repository(repository, files: { "LICENSE" => License.find_by!(key: "mit").template })

      rules = Repositories::DetectLicense.call(repository).rules

      assert_equal %i[permissions conditions limitations], rules.keys
      assert_includes rules[:permissions], "commercial-use"
    end
  end
end
