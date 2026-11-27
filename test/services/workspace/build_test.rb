require "test_helper"

class Workspace::BuildTest < ActiveSupport::TestCase
  setup { @user = users(:one) }

  test "uses real records and no placeholder data without a scenario" do
    state = Workspace::Build.call(user: @user)

    assert state.first_run?
    assert_not state.preview?
    assert_empty state.items
    assert_empty state.runs
    assert_equal ["Meelting"], state.repositories.map(&:name)
    assert state.repositories.all?(&:real)
    assert_equal :none, state.repositories.first.mirror
  end

  test "checks the token step with real access tokens" do
    state = Workspace::Build.call(user: @user)

    assert state.checklist.find { |step_| step_.key == :access_token }.done
    assert_equal :repository, state.current_step.key
    assert_equal 1, state.completed_steps
  end

  test "counts everything that does not exist yet as not done" do
    state = Workspace::Build.call(user: users(:two))
    done = state.checklist.select(&:done).map(&:key)

    assert_equal [:access_token], done
    assert_not state.checklist.find { |step_| step_.key == :repository }.done
  end

  test "inbox scenario sorts items by waiting time, oldest first" do
    state = Workspace::Build.call(user: @user, scenario: :inbox)

    assert_equal %i[failed merge approve], state.items.map(&:kind)
    assert_equal state.items.map(&:waiting_seconds).sort.reverse, state.items.map(&:waiting_seconds)
    assert_equal 3, state.inbox_count
    assert_equal 1, state.runs_count
    assert state.preview?
    assert_not state.settings_warning?
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
    assert state.settings_warning?
  end

  test "empty scenario falls back to placeholder repositories when the user has none" do
    @user.repositories.destroy_all
    state = Workspace::Build.call(user: @user, scenario: :empty)

    assert_equal %w[meelting veta], state.repositories.map(&:slug)
    assert_nil state.assign_issue_repository
  end

  test "first run scenario forces the checklist without placeholder blocks" do
    state = Workspace::Build.call(user: @user, scenario: :first_run)

    assert state.first_run?
    assert_empty state.items
    assert_empty state.runs
  end
end
