class HomeController < ApplicationController
  def show
    @inbox = Inbox::Sample.new(params[:item])
  end
end
