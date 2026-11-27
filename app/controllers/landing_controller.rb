class LandingController < ApplicationController
  layout "landing"

  allow_unauthenticated_access only: :show

  def show
  end
end
