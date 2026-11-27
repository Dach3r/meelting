module Repositories
  class SectionsController < ApplicationController
    PHASES = { "code" => 1, "issues" => 2, "pull_requests" => 2, "runs" => 3, "memory" => 4 }.freeze

    def show
      @repository = Current.user.repositories.active.find_by!(slug: params[:repository_slug])
      @section = params[:section]
      @phase = PHASES.fetch(@section)
    end
  end
end
