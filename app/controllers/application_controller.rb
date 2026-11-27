class ApplicationController < ActionController::Base
  include Authentication
  include Previewable
  stale_when_importmap_changes
end
