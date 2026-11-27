module Auth
  class UsernameAvailabilitiesController < ApplicationController
    allow_unauthenticated_access only: :show
    rate_limit to: 60, within: 1.minute, only: :show

    def show
      @errors = User.username_errors(params[:username].to_s)
    end
  end
end
