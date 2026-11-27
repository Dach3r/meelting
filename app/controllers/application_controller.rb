class ApplicationController < ActionController::Base
  include Authentication
  stale_when_importmap_changes

  helper_method :inbox_counts

  private

  def inbox_counts
    Inbox::Sample.new.counts
  end
end
