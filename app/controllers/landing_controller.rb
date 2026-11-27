class LandingController < ApplicationController
  allow_unauthenticated_access only: :show

  def show
    render "pages/landing"
  end
end
