class RepositoriesController < ApplicationController
  def index
    @repositories = Current.user.repositories.active.order(:name)
  end

  def new
    @repository = Current.user.repositories.new
  end

  def create
    @repository = Current.user.repositories.new(repository_params)

    if @repository.save
      redirect_to repositories_path, notice: t(".created")
    else
      render :new, status: :unprocessable_entity
    end
  end

  private

  def repository_params
    params.expect(repository: %i[name slug description visibility license_id default_branch])
  end
end
