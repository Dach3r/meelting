class RepositoriesController < ApplicationController
  def index
    @repositories = Current.user.repositories.active.order(:name)
  end

  def new
    @repository = Current.user.repositories.new
    @taken_slugs = Current.user.repositories.pluck(:slug)
  end

  def create
    @repository = Current.user.repositories.new(repository_params)

    if @repository.save
      redirect_to repository_section_path(@repository.slug, :code), notice: t(".created")
    else
      @taken_slugs = Current.user.repositories.pluck(:slug)
      render :new, status: :unprocessable_entity
    end
  end

  private

  def repository_params
    params.expect(repository: %i[name description visibility license_id default_branch])
  end
end
