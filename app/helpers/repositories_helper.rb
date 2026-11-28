module RepositoriesHelper
  REPOSITORY_WIZARD_STEPS = [
    %i[name slug description],
    %i[visibility license license_id],
    %i[default_branch]
  ].freeze

  def repository_wizard_step(repository)
    attribute = repository.errors.attribute_names.first
    index = REPOSITORY_WIZARD_STEPS.index { |attributes| attributes.include?(attribute) }
    index ? index + 1 : 1
  end
end
