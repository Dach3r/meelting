require "test_helper"

class RepositoryTest < ActiveSupport::TestCase
  def build(**attrs)
    Repository.new({ user: users(:one), name: "Other", slug: "other" }.merge(attrs))
  end

  test "valid repository with defaults" do
    repository = build
    assert repository.valid?
    assert repository.visibility_private?
    assert_equal "main", repository.default_branch
  end

  test "requires name and slug" do
    repository = Repository.new(user: users(:one))
    assert_not repository.valid?
    assert repository.errors[:name].any?
    assert repository.errors[:slug].any?
  end

  test "normalizes slug" do
    assert_equal "other", build(slug: "  Other ").slug
  end

  test "rejects invalid slug format" do
    ["-bad", "has space", "bad/slash", ".hidden", "ñandú"].each do |slug|
      assert_not build(slug: slug).valid?, "#{slug} should be invalid"
    end
  end

  test "rejects duplicate slug for the same user regardless of case" do
    repository = build(slug: repositories(:meelting).slug.upcase)
    assert_not repository.valid?
    assert repository.errors[:slug].any?
  end

  test "allows the same slug for different users" do
    assert build(user: users(:two), slug: repositories(:meelting).slug).valid?
  end

  test "database enforces slug uniqueness per user" do
    duplicate = build(slug: repositories(:meelting).slug)
    assert_raises(ActiveRecord::RecordNotUnique) { duplicate.save!(validate: false) }
  end

  test "description max length" do
    assert_not build(description: "a" * 351).valid?
    assert build(description: "a" * 350).valid?
  end

  test "license is optional" do
    assert build(license: nil).valid?
  end

  test "rejects inactive license" do
    repository = build(license: licenses(:retired))
    assert_not repository.valid?
    assert repository.errors[:license].any?
  end

  test "rejects unknown visibility" do
    assert_raises(ArgumentError) { build(visibility: "internal") }
  end

  test "archive and unarchive" do
    repository = repositories(:meelting)
    assert_not repository.archived?

    repository.archive!
    assert repository.archived?
    assert_includes Repository.archived, repository
    assert_not_includes Repository.active, repository

    repository.unarchive!
    assert_not repository.archived?
  end

  test "touch_pushed sets pushed_at" do
    repository = repositories(:meelting)
    assert_nil repository.pushed_at
    repository.touch_pushed
    assert_not_nil repository.reload.pushed_at
  end

  test "stores the bare repository under the git root by id" do
    repository = repositories(:meelting)

    assert_equal Rails.configuration.x.git_root.join("#{repository.id}.git"), repository.git_path
  end

  test "finds a file on the default branch of the bare repository" do
    repository = repositories(:meelting)

    with_git_root do
      assert_not repository.file_on_default_branch?(".project/overview.md")

      create_bare_repository(repository, files: { ".project/overview.md" => "# Overview" })
      assert repository.file_on_default_branch?(".project/overview.md")
      assert_not repository.file_on_default_branch?(".project/missing.md")
    end
  end

  test "ignores files on other branches" do
    repository = repositories(:meelting)

    with_git_root do
      create_bare_repository(repository, files: { ".project/overview.md" => "# Overview" }, branch: "draft")
      assert_not repository.file_on_default_branch?(".project/overview.md")
    end
  end

  test "rejects default branch names that git would read as options or invalid refs" do
    repository = repositories(:meelting)

    with_git_root do
      create_bare_repository(repository, files: { ".project/overview.md" => "# Overview" })

      ["--output=/tmp/x", "-h", "bad..name"].each do |branch_|
        repository.default_branch = branch_
        assert_not repository.file_on_default_branch?(".project/overview.md"), branch_
      end
    end
  end
end
