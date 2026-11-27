module Previewable
  extend ActiveSupport::Concern

  included do
    helper_method :workspace
  end

  private
  # ?preview=inbox|empty|first_run swaps in placeholder data, in development only.
  def preview_scenario
    return unless Rails.env.development?

    scenario = params[:preview].to_s
    scenario.to_sym if Workspace::PlaceholderData::SCENARIOS.map(&:to_s).include?(scenario)
  end

  def workspace
    @workspace ||= Workspace::Build.call(user: Current.user, scenario: preview_scenario)
  end
end
