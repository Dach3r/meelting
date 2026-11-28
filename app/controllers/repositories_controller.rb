class RepositoriesController < ApplicationController
  def index
    @repositories = Current.user.repositories.active.order(:name)
  end

  def new
    @repository = Current.user.repositories.new
    @taken_slugs = Current.user.repositories.pluck(:slug)
    @licenses = License.selectable
  end

  def create
    @repository = Current.user.repositories.new(repository_params)

    if Repositories::Create.call(@repository)
      redirect_to repository_path(Current.user.username, @repository.slug), notice: t(".created")
    else
      @taken_slugs = Current.user.repositories.pluck(:slug)
      @licenses = License.selectable
      render :new, status: :unprocessable_entity
    end
  end

  private

  def repository_params
    params.expect(repository: %i[name description visibility license_id default_branch])
  end
end
