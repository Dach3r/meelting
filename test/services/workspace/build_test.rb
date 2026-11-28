require "test_helper"

class Workspace::BuildTest < ActiveSupport::TestCase
  setup { @user = users(:one) }

  test "uses real records and no placeholder data without a scenario" do
    state = Workspace::Build.call(user: @user)

    assert_not state.no_repositories?
    assert_not state.preview
    assert_empty state.items
    assert_empty state.runs
    assert_equal ["Meelting"], state.repositories.map(&:name)
    assert state.repositories.all?(&:real)
    assert_equal :none, state.repositories.first.mirror
  end

  test "shows the home without repositories when the user has none" do
    @user.repositories.destroy_all
    state = Workspace::Build.call(user: @user)

    assert state.no_repositories?
    assert_empty state.repositories
  end

  test "shows the inbox with a repository, whether or not it was pushed" do
    assert_not repositories(:meelting).pushed?
    state = Workspace::Build.call(user: @user)

    assert_not state.no_repositories?
    assert_nil state.banner_problem
  end

  test "shows no model key problem until Phase 3" do
    @user.provider_credentials.create!(api_key: "sk", status: :invalid)

    assert_nil Workspace::Build.call(user: @user).banner_problem
  end

  test "inbox scenario sorts items by waiting time, oldest first" do
    state = Workspace::Build.call(user: @user, scenario: :inbox)

    assert_equal %i[failed merge approve], state.items.map(&:kind)
    assert_equal state.items.map(&:waiting_seconds).sort.reverse, state.items.map(&:waiting_seconds)
    assert_equal 3, state.inbox_count
    assert_equal 1, state.runs_count
    assert state.preview
    assert_nil state.banner_problem
    assert_equal %i[ok], state.repositories.map(&:mirror).uniq
  end

  test "every inbox item has at most one flag and the right flags per kind" do
    flags = Workspace::PlaceholderData::ITEMS.to_h { |item_| [item_.kind, item_.flag] }

    assert_nil flags[:failed]
    assert_includes [nil, :tests_failed], flags[:approve]
    assert_includes [nil, :modifies_project, :has_conflicts], flags[:merge]
  end

  test "empty scenario has no items or runs and a failing mirror problem" do
    state = Workspace::Build.call(user: @user, scenario: :empty)

    assert_empty state.items
    assert_empty state.runs
    assert_equal :mirror_failing, state.banner_problem.kind
    assert_equal :failing, state.repositories.last.mirror
  end

  test "empty scenario falls back to placeholder repositories when the user has none" do
    @user.repositories.destroy_all
    state = Workspace::Build.call(user: @user, scenario: :empty)

    assert_equal %w[meelting veta], state.repositories.map(&:slug)
    assert_nil state.assign_issue_repository
  end

  test "no repositories scenario forces the empty home without placeholder blocks" do
    state = Workspace::Build.call(user: @user, scenario: :no_repositories)

    assert state.no_repositories?
    assert_empty state.items
    assert_empty state.runs
  end
end
