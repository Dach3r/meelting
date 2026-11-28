require "test_helper"

class RepositoryTest < ActiveSupport::TestCase
  def build(**attrs)
    Repository.new({ user: users(:one), name: "Other" }.merge(attrs))
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

  test "generates the slug from the name" do
    { "Hola mundo" => "hola-mundo", "Los de la culpa 2026" => "los-de-la-culpa-2026", "Ñandú  Café!" => "nandu-cafe" }.each do |name, slug|
      repository = build(name: name)
      repository.validate
      assert_equal slug, repository.slug
    end
  end

  test "ignores a submitted slug" do
    repository = build(slug: "custom")
    repository.validate
    assert_equal "other", repository.slug
  end

  test "regenerates the slug when the name changes" do
    repository = repositories(:meelting)
    repository.update!(name: "Renamed Forge")
    assert_equal "renamed-forge", repository.slug
  end

  test "rejects a name without letters or numbers" do
    repository = build(name: "!!!")
    assert_not repository.valid?
    assert repository.errors[:slug].any?
  end

  test "rejects duplicate slug for the same user regardless of case" do
    repository = build(name: "MEELTING")
    assert_not repository.valid?
    assert repository.errors[:slug].any?
  end

  test "allows the same slug for different users" do
    assert build(user: users(:two), name: "Meelting").valid?
  end

  test "database enforces slug uniqueness per user" do
    duplicate = build(name: "Meelting")
    duplicate.validate
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

  test "active excludes archived repositories" do
    repository = repositories(:meelting)
    assert_includes Repository.active, repository

    repository.update!(archived_at: Time.current)
    assert_not_includes Repository.active, repository
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
end
