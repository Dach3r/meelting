module Navigation
  class ExpansionsController < ApplicationController
    def update
      slug = params[:repository_slug].to_s

      if slug.length <= 100 && slug.match?(Repository::SLUG_FORMAT)
        session[:expanded_repository] = slug
      else
        session.delete(:expanded_repository)
      end

      head :no_content
    end
  end
end
