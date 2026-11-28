module RepositoryScoped
  extend ActiveSupport::Concern

  private

  def find_repository
    raise ActiveRecord::RecordNotFound unless params[:username] == Current.user.username

    Current.user.repositories.active.find_by!(slug: params[:repository_slug])
  end
end
