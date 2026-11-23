import sys
from PIL import Image
tag=sys.argv[1]
ims=[Image.open(f't-{tag}-{i}.png').crop((330,83,1920,1150)) for i in range(4)]
w,h=ims[0].size
out=Image.new('RGB',(w*2,h*2))
for i,im in enumerate(ims): out.paste(im,((i%2)*w,(i//2)*h))
out.resize((w,h)).save(f'grid-{tag}.png')
