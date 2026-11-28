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

  test "lists four steps with the first issue disabled and nothing done on a new account" do
    state = Workspace::Build.call(user: @user)

    assert_equal %i[repository model_key overview first_issue], state.checklist.map(&:key)
    assert_equal 0, state.completed_steps
    assert_equal :repository, state.current_step.key
    assert_not state.checklist.last.available
    assert_equal repositories(:meelting), state.push_repository
  end

  test "checks the push step when a repository has been pushed" do
    repositories(:meelting).touch_pushed
    state = Workspace::Build.call(user: @user)

    assert state.checklist.first.done
    assert_equal :model_key, state.current_step.key
  end

  test "checks the overview step from the default branch of a pushed repository" do
    repository = repositories(:meelting)

    with_git_root do
      create_bare_repository(repository, files: { ".project/overview.md" => "# Overview" })
      assert_not Workspace::Build.call(user: @user).checklist.find { |step_| step_.key == :overview }.done

      repository.touch_pushed
      assert Workspace::Build.call(user: @user).checklist.find { |step_| step_.key == :overview }.done
    end
  end

  test "hides the missing model key warning while the checklist is shown" do
    state = Workspace::Build.call(user: @user)

    assert state.first_run?
    assert_not state.settings_warning?
    assert_nil state.banner_problem
  end

  test "marks onboarding complete once every step is done" do
    completed = Class.new(Workspace::Build) do
      private
      def checklist
        [Workspace::ChecklistStep.new(key: :repository, done: true)]
      end
    end
    state = completed.call(user: @user)

    assert_not state.first_run?

    assert @user.reload.onboarding_completed?
  end

  test "does not complete onboarding while a step is pending" do
    Workspace::Build.call(user: @user)

    assert_not @user.reload.onboarding_completed?
  end

  test "never shows the checklist again after onboarding, even if a step stops being true" do
    @user.complete_onboarding!
    state = Workspace::Build.call(user: @user)

    assert_not state.first_run?
    assert_empty state.checklist
    assert_nil state.push_repository
    assert_equal :missing_model_key, state.banner_problem.kind
    assert state.settings_warning?
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
