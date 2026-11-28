module Repositories
  class BlobsController < ApplicationController
    include RepositoryScoped

    MAX_BYTES = 1.megabyte

    before_action { request.format = :html }

    def show
      @repository = find_repository
      @path = params[:path]
      content = @repository.head_sha && @repository.git.blob_at(@repository.head_sha, @path)
      raise ActiveRecord::RecordNotFound unless content

      @text = content.force_encoding(Encoding::UTF_8).scrub if content.bytesize <= MAX_BYTES
      @license = license_for(@path)
    end

    private
    def license_for(path)
      license = Repositories::DetectLicense.call(@repository)
      license if license && !license.other? && license.path == path
    end
  end
end
