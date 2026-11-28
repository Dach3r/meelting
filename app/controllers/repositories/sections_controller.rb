module Repositories
  class SectionsController < ApplicationController
    include RepositoryScoped

    PHASES = { "code" => 1, "issues" => 2, "pull_requests" => 2, "runs" => 3, "memory" => 4 }.freeze

    def show
      @repository = find_repository
      @section = params[:section]
      @phase = PHASES.fetch(@section)
      return unless @section == "code"

      @empty = !@repository.initialized?
      @license = Repositories::DetectLicense.call(@repository) unless @empty
    end
  end
end
